# Wolf Citadel stock inspection

Wolf Citadel (SH161) is a catalogue station. The station Showroom displays 2,052 detailed faces with the original palette and material assignments. The native mesh matches the original L337 archive in its complete vertex, triangle and appearance tables. The original physical record gives a scale of 1000000/22443 metres per source unit. The stock package applies that scale once to the original positions.

The stock registry uses the schema-2 inspection loader. The 788-face original exceeds the 512-face flight limit. Studio refuses flight export and the archive declares no engineering or flight specification. The original engineering record remains with the sources.

## Reproduce

```sh
python3 tools/catalogue_artwork.py integration/sh161-wolf-citadel/SOURCE.json --check
```

The source manifest records the immutable L337 revision and individual input hashes. The imported MIT notice is in the source folder. [Original inputs](../assets/fleet/sh161) · [Inspection package](../integration/sh161-wolf-citadel/ORIGINAL-INSPECTION.json)
