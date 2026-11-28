# Larvaal Probe stock inspection

Larvaal Probe (SH162) is a catalogue other. The station Showroom displays 1,524 detailed faces with the original palette and material assignments. The native mesh matches the original L337 archive in its complete vertex, triangle and appearance tables. The original physical record gives a scale of 12000000/2058667 metres per source unit. The stock package applies that scale once to the original positions.

The stock registry uses the schema-2 inspection loader. The 596-face original exceeds the 512-face flight limit. Studio refuses flight export and the archive declares no engineering or flight specification. The original engineering record remains with the sources.

## Reproduce

```sh
python3 tools/catalogue_artwork.py integration/sh162-larvaal-probe/SOURCE.json --check
```

The source manifest records the immutable L337 revision and individual input hashes. The imported MIT notice is in the source folder. [Original inputs](../assets/fleet/sh162) · [Inspection package](../integration/sh162-larvaal-probe/ORIGINAL-INSPECTION.json)
