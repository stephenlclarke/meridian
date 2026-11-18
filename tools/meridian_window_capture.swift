// Execution pending an explicit human exception for alternate UI capture technology.
// Read-only, one exact test window. No input, activation, picker, permission request or fallback.
import Foundation
import Dispatch
@preconcurrency import AppKit
@preconcurrency import ScreenCaptureKit
import CoreGraphics
import CoreVideo
import ImageIO
import UniformTypeIdentifiers
import CryptoKit
import Darwin

private let allowedBundle = "io.xyzzy.meridian.review-v2.ad5b9f9"
private let maxImageBytes = 16 * 1024 * 1024
private enum Rejection: Error { case reason(String) }
private func require(_ condition: Bool, _ reason: String) throws {
    if !condition { throw Rejection.reason(reason) }
}
private struct Bounds: Codable, Sendable {
    let x: Double, y: Double, width: Double, height: Double
    init(_ rect: CGRect) { x = rect.origin.x; y = rect.origin.y; width = rect.width; height = rect.height }
    func matches(_ other: Bounds) -> Bool {
        [x, y, width, height, other.x, other.y, other.width, other.height].allSatisfy(\.isFinite)
            && abs(x - other.x) <= 0.5 && abs(y - other.y) <= 0.5
            && abs(width - other.width) <= 0.5 && abs(height - other.height) <= 0.5
    }
    var valid: Bool { [x, y, width, height].allSatisfy(\.isFinite) && width > 0 && height > 0 && width <= 4096 && height <= 4096 }
}
private struct Scope: Decodable, Sendable {
    let pid: Int32
    let bundle_id: String
    let bounds: Bounds
    let human_exception_reference: String
}
private struct WindowRecord: Codable, Sendable {
    let pid: Int32, window_id: UInt32, window_layer: Int
    let bundle_id: String, title: String
    let on_screen: Bool, active: Bool, fully_on_one_display: Bool, covers_display: Bool
    let bounds: Bounds
}
private struct Input: Decodable, Sendable { let scope: Scope; let selected: WindowRecord? }
private struct Enumeration: Encodable {
    let status = "ok", api = "SCShareableContent: target PID/bundle metadata only"
    let pid: Int32, bundle_id: String, windows: [WindowRecord]
    let started_uptime_ns: UInt64, ended_uptime_ns: UInt64
}
private struct CaptureReply: Encodable {
    let status = "ok", api = "SCScreenshotManager.captureImage; desktopIndependentWindow only"
    let pid: Int32, bundle_id: String
    let chosen_before: WindowRecord, chosen_after: WindowRecord
    let capture_started_uptime_ns: UInt64, capture_ended_uptime_ns: UInt64, postcheck_uptime_ns: UInt64
    let image_width: Int, image_height: Int, image_bits_per_pixel: Int, image_bytes_per_row: Int
    let point_pixel_scale: Float, filter_content_rect: Bounds
    let png_bytes: Int, png_sha256: String, png_base64: String
    let native_drawable_presented = "UNOBSERVABLE"
    let association = "window identity rechecked before/after; monotonic bracket only, not atomic F8/drawable/display identity"
}
private struct Failure: Encodable { let status = "rejected"; let reason: String; let ended_uptime_ns: UInt64 }

@main
private struct NativeProvider {
    @MainActor
    static func emit<T: Encodable>(_ value: T) throws {
        let data = try JSONEncoder().encode(value)
        try require(data.count <= 24 * 1024 * 1024, "bounded_response_exceeded")
        try FileHandle.standardOutput.write(contentsOf: data)
        try FileHandle.standardOutput.write(contentsOf: Data([10]))
    }
    @MainActor
    static func validate(_ scope: Scope) throws {
        try require(scope.bundle_id == allowedBundle && scope.pid > 1 && scope.bounds.valid, "exact_test_scope_required")
        try require(scope.human_exception_reference.hasPrefix("human-approved:") && scope.human_exception_reference.count >= 20 && scope.human_exception_reference.count <= 2048 && !scope.human_exception_reference.uppercased().contains("PENDING"), "human_exception_not_recorded")
        // This reference correlates an externally granted exception. It cannot grant one.
        try require(CGPreflightScreenCaptureAccess(), "screen_capture_permission_unavailable_no_request")
        guard let app = NSRunningApplication(processIdentifier: scope.pid), !app.isTerminated,
              app.bundleIdentifier == allowedBundle else { throw Rejection.reason("live_process_bundle_mismatch") }
    }
    @MainActor
    static func content(_ scope: Scope) async throws -> SCShareableContent {
        try validate(scope)
        return try await SCShareableContent.excludingDesktopWindows(true, onScreenWindowsOnly: true)
    }
    @MainActor
    static func records(_ content: SCShareableContent, _ scope: Scope) throws -> [(SCWindow, WindowRecord)] {
        let owned = content.windows.filter { $0.owningApplication?.processID == scope.pid }
        try require(owned.count <= 16, "excess_owned_window_metadata")
        return try owned.map { window in
            guard let owner = window.owningApplication, owner.bundleIdentifier == allowedBundle else {
                throw Rejection.reason("window_owner_bundle_mismatch")
            }
            let frame = window.frame
            let contained = content.displays.contains { display in
                let d = display.frame
                return frame.minX >= d.minX && frame.minY >= d.minY && frame.maxX <= d.maxX && frame.maxY <= d.maxY
            }
            let covers = content.displays.contains { Bounds($0.frame).matches(Bounds(frame)) }
            return (window, WindowRecord(pid: owner.processID, window_id: window.windowID,
                window_layer: window.windowLayer, bundle_id: owner.bundleIdentifier,
                title: String((window.title ?? "").prefix(512)), on_screen: window.isOnScreen,
                active: window.isActive, fully_on_one_display: contained, covers_display: covers, bounds: Bounds(frame)))
        }
    }
    @MainActor
    static func choose(_ entries: [(SCWindow, WindowRecord)], _ scope: Scope) throws -> (SCWindow, WindowRecord) {
        let normal = entries.filter { $0.1.window_layer == 0 && $0.1.on_screen }
        try require(normal.count == 1, "missing_or_ambiguous_visible_normal_window")
        let chosen = normal[0]
        try require(chosen.1.active && chosen.1.window_id != 0 && chosen.1.bounds.matches(scope.bounds)
                    && chosen.1.fully_on_one_display && !chosen.1.covers_display, "window_bounds_visibility_or_fullscreen_mismatch")
        return chosen
    }
    @available(macOS 14.2, *)
    @MainActor
    static func capture(_ input: Input) async throws {
        guard let selected = input.selected else { throw Rejection.reason("selected_cgwindowid_required") }
        let initial = try records(try await content(input.scope), input.scope)
        let (window, before) = try choose(initial, input.scope)
        try require(before.window_id == selected.window_id && before.pid == selected.pid
                    && before.bundle_id == selected.bundle_id && before.bounds.matches(selected.bounds), "selection_changed_before_capture")
        // SCWindow.windowID is documented as CGWindowID. No winit pointer is used.
        let filter = SCContentFilter(desktopIndependentWindow: window)
        let info = SCShareableContent.info(for: filter)
        let scale = info.pointPixelScale
        let width = ceil(Double(info.contentRect.width) * Double(scale))
        let height = ceil(Double(info.contentRect.height) * Double(scale))
        try require(scale.isFinite && scale > 0 && scale <= 4 && width.isFinite && height.isFinite
                    && width > 0 && height > 0 && width <= 4096 && height <= 4096
                    && width * height * 4 <= Double(maxImageBytes), "native_capture_dimensions_exceed_bound")
        let config = SCStreamConfiguration()
        config.width = Int(width); config.height = Int(height)
        config.pixelFormat = kCVPixelFormatType_32BGRA
        config.showsCursor = false; config.capturesAudio = false
        config.scalesToFit = false; config.ignoreShadowsSingleWindow = true
        config.ignoreGlobalClipSingleWindow = false; config.includeChildWindows = false
        // No display/application filter, picker, alternate target or input API exists here.
        let started = DispatchTime.now().uptimeNanoseconds
        let image = try await SCScreenshotManager.captureImage(contentFilter: filter, configuration: config)
        let ended = DispatchTime.now().uptimeNanoseconds
        let (_, after) = try choose(try records(try await content(input.scope), input.scope), input.scope)
        try require(after.window_id == before.window_id && after.bounds.matches(before.bounds), "window_changed_during_capture_no_artifact")
        try require(image.width == config.width && image.height == config.height && image.bitsPerPixel == 32
                    && image.bytesPerRow <= maxImageBytes / image.height, "returned_image_dimensions_or_size_mismatch")
        let encoded = NSMutableData()
        guard let destination = CGImageDestinationCreateWithData(encoded as CFMutableData, UTType.png.identifier as CFString, 1, nil) else {
            throw Rejection.reason("png_destination_failed")
        }
        CGImageDestinationAddImage(destination, image, nil)
        try require(CGImageDestinationFinalize(destination) && encoded.length > 0 && encoded.length <= maxImageBytes, "png_encoding_failed_or_oversize")
        let png = encoded as Data
        let sha = SHA256.hash(data: png).map { String(format: "%02x", $0) }.joined()
        try emit(CaptureReply(pid: input.scope.pid, bundle_id: allowedBundle, chosen_before: before, chosen_after: after,
            capture_started_uptime_ns: started, capture_ended_uptime_ns: ended,
            postcheck_uptime_ns: DispatchTime.now().uptimeNanoseconds,
            image_width: image.width, image_height: image.height, image_bits_per_pixel: image.bitsPerPixel,
            image_bytes_per_row: image.bytesPerRow, point_pixel_scale: scale, filter_content_rect: Bounds(info.contentRect),
            png_bytes: png.count, png_sha256: sha, png_base64: png.base64EncodedString()))
    }
    @MainActor
    static func main() async {
        do {
            try require(CommandLine.arguments.count == 2 && ["enumerate", "capture"].contains(CommandLine.arguments[1]), "one_supported_mode_required")
            let data = FileHandle.standardInput.readData(ofLength: 65537)
            try require(data.count <= 65536, "bounded_input_exceeded")
            let input = try JSONDecoder().decode(Input.self, from: data)
            try require(input.scope.human_exception_reference.hasPrefix("human-approved:"), "human_exception_not_recorded")
            if #available(macOS 14.2, *) {
                if CommandLine.arguments[1] == "enumerate" {
                    let started = DispatchTime.now().uptimeNanoseconds
                    let entries = try records(try await content(input.scope), input.scope)
                    try emit(Enumeration(pid: input.scope.pid, bundle_id: allowedBundle, windows: entries.map(\.1),
                        started_uptime_ns: started, ended_uptime_ns: DispatchTime.now().uptimeNanoseconds))
                } else { try await capture(input) }
            } else { throw Rejection.reason("macOS_14_2_required_no_fallback") }
        } catch {
            let reason: String
            if let policy = error as? Rejection {
                switch policy { case .reason(let value): reason = value }
            } else { let native = error as NSError; reason = "native_api_error:\(native.domain):\(native.code)" }
            try? emit(Failure(reason: String(reason.prefix(512)), ended_uptime_ns: DispatchTime.now().uptimeNanoseconds))
            exit(2)
        }
    }
}
