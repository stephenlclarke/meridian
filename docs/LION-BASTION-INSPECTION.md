# Lion Bastion stock inspection

Lion Bastion (SH156) is a catalogue platform. The station Showroom displays 4,072 detailed faces with the original palette and material assignments. The native mesh matches the original L337 archive in its complete vertex, triangle and appearance tables. The original physical record gives a scale of 90000/1699 metres per source unit. The stock package applies that scale once to the original positions.

The stock registry uses the schema-2 inspection loader. The 904-face original exceeds the 512-face flight limit. Studio refuses flight export and the archive declares no engineering or flight specification. The original engineering record remains with the sources.

## Reproduce

```sh
python3 tools/catalogue_artwork.py integration/sh156-lion-bastion/SOURCE.json --check
```

The source manifest records the immutable L337 revision and individual input hashes. The imported MIT notice is in the source folder. [Original inputs](../assets/fleet/sh156) · [Inspection package](../integration/sh156-lion-bastion/ORIGINAL-INSPECTION.json)
