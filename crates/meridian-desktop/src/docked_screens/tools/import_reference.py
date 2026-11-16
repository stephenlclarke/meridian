"""Rebuild display-only original L337 data from an explicit immutable Git object store."""
import argparse,hashlib,json,subprocess,tomllib
from pathlib import Path
REVISION="a11fe16924d87d06021816f9037952afcbac08db"
ROOT=Path(__file__).resolve().parents[1]

def build(reference):
    sources=[]
    def read(path):
        raw=subprocess.check_output(["git","show",REVISION+":"+path],cwd=reference)
        sources.append({"path":path,"bytes":len(raw),"sha256":hashlib.sha256(raw).hexdigest()})
        return raw
    def text(value):
        if isinstance(value,bool): value="true" if value else "false"
        elif isinstance(value,list):value=", ".join(map(str,value)) or "[]"
        else:value=str(value)
        if any(c in value for c in "\t\r\n"):raise ValueError("Display TSV field contains a separator")
        return value
    world=json.loads(read("Sources/L337Core/Resources/OriginalWorlds/catalogue.json"))
    keys=["galaxy","system","name","xLY","yLY","economyID","economy","governmentID","government","technology","populationBillions","productivityBillionCR","radiusKM","ringed","surface","climate"]
    worlds=["\t".join(text(row[k]) for k in keys) for row in world["systems"]]
    assert len(worlds)==2048
    ships=[];fields=[];kinds={}
    for index in range(1,172):
        path=f"docs/fleet-design-v2/ships/SH{index:03}.toml";d=tomllib.loads(read(path).decode());kinds[d["objectKind"]]=kinds.get(d["objectKind"],0)+1
        ships.append("\t".join(text(x) for x in [d["id"],d["name"],d["family"],d["objectKind"],d["classification"],d["access"]["playerFlyable"],d["access"]["purchasePolicy"],d["decisionStatus"]]))
        for group in ["performance","accommodation","fitting","geometry","medusaConfigurations"]:
            for key,value in d.get(group,{}).items():
                if isinstance(value,dict):continue
                fields.append("\t".join(text(x) for x in [d["id"],group,key,value]))
    assert kinds=={"ship":162,"station":6,"capsule":1,"platform":1,"other":1}
    outputs={"worlds.tsv":"\n".join(worlds)+"\n","objects.tsv":"\n".join(ships)+"\n","specifications.tsv":"\n".join(fields)+"\n"}
    for name,value in outputs.items():(ROOT/"data"/name).write_text(value)
    for name in ["ScreenNavigation","StationScreenGuide","DockedChartDisplay","GalaxyBrowsing","SystemDisplay","ShipShowroom","ShowroomDisplay","ShowroomGeometry","EquipmentDisplay","EquipmentGroups","EquipmentRuntimeCatalog","EquipmentModuleReadout","EquipmentModuleTemplates","MarketDisplayOrder","MarketInteraction","StationDisplay","ChartDisplay","CompactScreens","GameTypography","Display","CockpitDisplay","OriginalFleetR3Data","OriginalFleetDefinition"]:read("Sources/L337Core/"+name+".swift")
    read("Sources/L337Mac/L337App.swift")
    license=read("LICENSE");(ROOT/"ORIGINAL-L337-MIT.txt").write_bytes(license)
    manifest={"revision":REVISION,"tree":"68fec574df12ddf74acb14747e0dd6839f478642","scope":"Exact display facts and screen behavior references. No meshes/textures/Elite/Oolite assets imported; no acquisition, runtime readiness or legal clearance inferred from authored catalogue.","objects":171,"ships":162,"otherObjectKinds":{k:v for k,v in kinds.items() if k!="ship"},"normalWorlds":2048,"hiddenPlughWorldsImported":False,"sources":sources,"outputs":[{"path":"data/"+n,"bytes":len(v.encode()),"sha256":hashlib.sha256(v.encode()).hexdigest()} for n,v in outputs.items()]}
    (ROOT/"SOURCE.json").write_text(json.dumps(manifest,indent=2)+"\n")
    print(json.dumps({"worlds":2048,"ships":162,"objects":171,"specifications":len(fields),"referenceFiles":len(sources)}))

def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reference", required=True, type=Path)
    build(parser.parse_args(argv).reference)


if __name__ == "__main__":
    main()
