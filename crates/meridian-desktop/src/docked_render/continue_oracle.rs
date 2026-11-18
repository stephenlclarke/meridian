//! Test-only independent V17 prefix walker. Does not call the private save codec or mutate World.
//! Layout: flight/persistence, navigation/persistence, freight/g02/persistence, equipment::encode.
struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl<'a> Cursor<'a> {
    fn take(&mut self, count: usize) -> &'a [u8] {
        let end = self
            .offset
            .checked_add(count)
            .unwrap_or_else(|| panic!("offset overflow"));
        let part = self
            .bytes
            .get(self.offset..end)
            .unwrap_or_else(|| panic!("truncated V17 prefix"));
        self.offset = end;
        part
    }
    fn skip(&mut self, count: usize) {
        self.take(count);
    }
    fn flag(&mut self) -> bool {
        match self.take(1)[0] {
            0 => false,
            1 => true,
            other => panic!("invalid option/boolean {other}"),
        }
    }
    fn u64(&mut self) -> u64 {
        u64::from_le_bytes(
            self.take(8)
                .try_into()
                .unwrap_or_else(|_| panic!("u64 width")),
        )
    }
}
fn checksum(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash = (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3);
    }
    hash
}
fn word(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(
        bytes[offset..offset + 8]
            .try_into()
            .unwrap_or_else(|_| panic!("word width")),
    )
}
fn review_offsets(bytes: &[u8], owner: u64) -> [usize; 2] {
    let mut r = Cursor { bytes, offset: 0 };
    assert_eq!(r.take(8), b"MERSAVE1");
    assert_eq!(r.take(2), 17u16.to_le_bytes());
    // Extract only the literal content identifier, not serializer code or a field offset.
    // Escapes/new format must fail explicitly instead of silently accepting another layout.
    let source = include_str!("../../../meridian-core/src/flight/persistence.rs");
    let section = source
        .split_once("const CONTENT:")
        .unwrap_or_else(|| panic!("content declaration changed"))
        .1
        .split_once(".as_bytes();")
        .unwrap_or_else(|| panic!("content declaration changed"))
        .0;
    let pieces: Vec<_> = section.split('"').skip(1).step_by(2).collect();
    assert!(!pieces.is_empty() && pieces.iter().all(|s| !s.contains('\\')));
    let content = pieces.concat();
    assert_eq!(r.take(content.len()), content.as_bytes());
    r.skip(8); // compiled content fingerprint
    r.skip((3 + 4 + 1) * 4 + 8 + 1 + 1); // position, quaternion, speed, tick, mode, refusal
    r.skip(8 + 2 * 8 + 4 * 8 + 1 + 2 + 8 + 8); // nav owner, ship, drive, system, visit, dock sequence
    if r.flag() {
        r.skip(8 + 2 * 8 + 1 + 2 + 3 * 8 + 1);
    } // optional DockContext
    if r.flag() {
        r.skip(1 + 2 + 4 + 2 + 1 + 2 + 8 + 4 * 8);
    } // optional PendingJump
    r.skip(3 * 8); // launches, jumps, fuel spent
    r.skip(3 * 8 + 4 + 4 * 2); // account owner/hull/revision, cash, cargo/food/reserved/fuel
    if r.flag() {
        r.skip(3 * 8); // starter fit owner/hull/inventory
        for _ in 0..8 {
            if r.flag() {
                r.skip(2 * 8 + 3); // item id/owner, component/pylon/origin
                if r.flag() {
                    r.skip(8);
                } // fitted hull
            }
        }
    }
    r.skip(16 * 8 * 2 + 1); // ordinary/reserved commodity quantities, legal status
    r.skip(8); // legacy cargo expansion revision
    if r.flag() {
        r.skip(5 * 8 + 1 + 2 + 4 + 1);
    } // optional paid cargo expansion receipt
    r.flag(); // fitted expansion
    r.skip(2 * 8); // ammunition and modular revisions
    r.flag();
    r.flag(); // active XYZZY, unlimited fuel
    assert_eq!(r.u64(), owner); // freight owner
    assert_eq!(r.u64(), 0); // no freight data revision in this public fixture
    assert_eq!(r.u64(), 0); // no freight action sequence
    assert_eq!(r.take(1), [0]); // no allocated jobs
    assert!(!r.flag() && !r.flag() && !r.flag()); // both jobs and clue absent
    let sequence = r.offset;
    r.u64();
    let epoch = r.offset;
    r.u64();
    [sequence, epoch]
}
pub(super) struct Expected {
    pub bytes: Vec<u8>,
    offsets: [usize; 2],
}
impl Expected {
    pub fn from_original(bytes: &[u8], owner: u64) -> Self {
        let offsets = review_offsets(bytes, owner);
        let body = bytes
            .len()
            .checked_sub(8)
            .unwrap_or_else(|| panic!("missing checksum"));
        assert_eq!(word(bytes, body), checksum(&bytes[..body]));
        let mut expected = bytes.to_vec();
        for offset in offsets {
            let next = word(bytes, offset)
                .checked_add(1)
                .unwrap_or_else(|| panic!("review sequence exhausted"));
            expected[offset..offset + 8].copy_from_slice(&next.to_le_bytes());
        }
        let digest = checksum(&expected[..body]);
        expected[body..].copy_from_slice(&digest.to_le_bytes());
        Self {
            bytes: expected,
            offsets,
        }
    }
    pub fn assert_actual(&self, original: &[u8], actual: &[u8]) {
        assert_eq!(actual, self.bytes); // full bytes, including checksum; no normalization or masking
        let [sequence, epoch] = self.offsets;
        assert_eq!(word(actual, sequence), word(original, sequence) + 1);
        assert_eq!(word(actual, epoch), word(original, epoch) + 1);
        assert_eq!(&actual[..sequence], &original[..sequence]);
        assert_eq!(&actual[sequence + 8..epoch], &original[sequence + 8..epoch]);
        assert_eq!(
            &actual[epoch + 8..actual.len() - 8],
            &original[epoch + 8..original.len() - 8]
        );
        assert_eq!(
            word(actual, actual.len() - 8),
            checksum(&actual[..actual.len() - 8])
        );
        assert_ne!(actual, original);
    }
}
