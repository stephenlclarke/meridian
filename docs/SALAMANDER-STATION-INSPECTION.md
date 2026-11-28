# Salamander Station stock inspection

Salamander Station (SH141) is a catalogue station. The station Showroom displays 6,252 detailed faces with the original palette and material assignments. The current original L337 station resource adds two opaque struts between the front ring and the gate sideposts. The selected 416-face mesh and the earlier 392-face archive are both retained. The current resource supplies the displayed face winding, material assignments and texture coordinates. The scale is one metre per unit.

Each local rounded support has a separating plane on one side of the original 64 m by 36 m axial aperture. Even after 20 mm rounding, the minimum separation from the full aperture is 980 mm. Five passage probes succeeded on the MBA and probes inside both gate struts hit solid geometry. This is a static inspection check. Dynamic docking remains a separate native test.

## Reproduce

```sh
python3 tools/prepare_salamander_station.py --check
```

The source manifest records the immutable L337 revision and individual input hashes. The imported MIT notice is in the source folder. [Original inputs](../assets/fleet/sh141) · [Inspection package](../integration/sh141-salamander-station/SALAMANDER-STUDIO.json)
