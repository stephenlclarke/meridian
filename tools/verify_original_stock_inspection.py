"""Compare a stock inspection archive to its immutable L337 and Meridian Git inputs."""
import argparse
from fractions import Fraction
import hashlib
import json
from pathlib import Path
import subprocess
import tomllib


LEGACY_REVISION = "d03f1b4ecfd7588bb6b27763e31c230ebe1fbf5a"


MERIDIAN_REPO = Path(__file__).resolve().parents[1]
GROUP_SOURCES = {
    "SH038": {
        "meridianRevision": "92cdee004accf6f8f1d6b27671391e3aa8391502",
        "path": "integration/sh038-vulture-mk2/ORIGINAL-ARCHIVE-CANDIDATE.json",
        "sha256": "830146297942971a87cef9c9d8527f15c48ccd1ecdd87d16b5c2b39ae1fcf208",
    },
}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def read_group_archive(pin):
    blob = f"{pin['meridianRevision']}:{pin['path']}"
    data = subprocess.check_output(["git", "-C", MERIDIAN_REPO, "cat-file", "blob", blob])
    require(hashlib.sha256(data).hexdigest() == pin["sha256"], "Meridian group archive hash mismatch")
    return json.loads(data)


def verify(package_path, repository):
    package = json.loads(package_path.read_bytes())
    provenance = package["provenance"]
    identity = package["id"]
    require(identity.startswith("SH") and len(identity) == 5 and identity[2:].isascii()
            and identity[2:].isdigit(), "Invalid stock identity")
    require(provenance["legacyRevision"] == LEGACY_REVISION, "Unexpected legacy revision")
    pin = GROUP_SOURCES.get(identity)
    require(pin is not None and provenance.get("groupSource") == pin, "Unexpected Meridian group source")
    archive = read_group_archive(pin)
    require(archive["id"] == identity, "Meridian group archive identity mismatch")
    for level in ("detail", "native"):
        groups = package[level]["groups"]
        require(isinstance(groups, list) and all(type(group) is int for group in groups)
                and groups == archive[level]["groups"], f"Changed {level} group membership")
    paths = {
        "resource": f"Sources/L337Core/Resources/Ships/{identity}.toml",
        "native": f"Sources/L337Core/Resources/OriginalFleetShowroomModels/{identity}.json",
        "ownership": f"docs/milestones/fleet-physical/distinct-creator-ownership-records/{identity}.json",
        "engineering": f"Sources/L337Core/Resources/OriginalFleetPhysical/{identity}.json",
        "contract": f"docs/fleet-design-v2/ships/{identity}.toml",
        "license": "LICENSE",
    }
    raw = {}
    for key, path in paths.items():
        content = subprocess.check_output(
            ["git", "-C", repository, "cat-file", "blob", f"{LEGACY_REVISION}:{path}"])
        pin = provenance["sourceFiles"][key]
        require(pin == {"path": path, "sha256": hashlib.sha256(content).hexdigest()},
                f"Source mismatch for {path}")
        raw[key] = content
    resource = tomllib.loads(raw["resource"].decode())
    contract = tomllib.loads(raw["contract"].decode())
    native = json.loads(raw["native"])
    physical = json.loads(raw["engineering"])
    authors = json.loads(raw["ownership"])
    require(all(source["id"] == identity for source in (resource, contract, native, physical, authors))
            and package["name"] == resource["name"] == contract["name"], "Source identity mismatch")
    require(native["sourceSHA256"] == physical["detailedSourceSHA256"] == authors["geometrySHA256"]
            == contract["source"]["geometrySHA256"] == provenance["geometrySHA256"],
            "Geometry binding mismatch")
    require(raw["license"].decode() == provenance["license"], "Source notice mismatch")
    require(resource["nativeDesign"] == native["nativeDesign"], "Archived native mesh mismatch")
    require(package["engineering"] is None and package["flight"]["maxSpeed"] is None
            and not package["clearances"] and package["schema"] == 2, "Expected an inspection archive")
    require(package["metresPerUnit"] == float(Fraction(physical["metresPerSourceUnitExact"])),
            "Original scale mismatch")
    for level, source_name, material_key in (("detail", "design", "faceMaterials"),
                                              ("native", "nativeDesign", "nativeFaceMaterials")):
        source = resource[source_name]
        mesh = package[level]
        require(mesh["vertices"] == source["vertices"] and mesh["triangles"] == source["faces"],
                f"Changed {level} geometry")
        require(package[material_key] == source["faceMaterials"], f"Changed {level} materials")
        require(source["license"] == source["appearanceLicense"] == "MIT"
                and not source.get("textures") and not source.get("faceTextures"),
                f"Unexpected {level} appearance")
        materials = [{"name": item["name"], "rgb": [(item["colour"] >> s) & 255 for s in (24, 16, 8)],
                      "alpha": item["colour"] & 255, "emissive": item["emissive"]}
                     for item in source["materials"]]
        require(materials == package["materials"], f"Changed {level} palette")
    print(f"{identity} matches all six original inputs, both meshes, both material streams and both Meridian group arrays.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("package", type=Path)
    parser.add_argument("legacy_repository", type=Path)
    args = parser.parse_args()
    verify(args.package, args.legacy_repository)
