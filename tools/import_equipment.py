"""Extract one original SH025 Large Cargo Bay contract from authenticated Git blobs."""
import argparse
from decimal import Decimal
import hashlib
import json
from pathlib import Path
import re
import struct
import subprocess
import tomllib
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
REVISION = 'a11fe16924d87d06021816f9037952afcbac08db'
TREE = '68fec574df12ddf74acb14747e0dd6839f478642'
SOURCES = {
    'authoring': ('Sources/L337Core/Resources/OriginalWorlds/authoring.toml',
               '47d83caae5a9b22a4a932f19c55f79c5cb5435b962826a0969ece096b11f80eb'),
    'worlds': ('Sources/L337Core/Resources/OriginalWorlds/catalogue.json',
               '120543eedc02a705cce0e37d026125bf926ec22651cff9337ca5d943f2ffd99d'),
    'ship': ('Sources/L337Core/Resources/Ships/SH025.toml',
               'aa8fd7b22b8881d6c4bb1017d981fe8a29907ee8243e4bfe0dfd9215a5c80883'),
    'r3': ('docs/fleet-design-v2/ships/SH025.toml',
               'a305641b7be704a2af0129e146d4ef1d72d165a1cbc7b935458c4c35aea14cc0'),
    'game': ('Sources/L337Core/Game.swift',
               'e5b4e30c835d460aa7ca6f77364db0aaeaf877bb9d73bacc162e9111b26e2e17'),
    'plugh': ('Sources/L337Core/Plugh.swift',
               '2c43ae0738fa93375eb51a4859e11bc33e7bf9e7496ff432f140e813baa6b8eb'),
    'runtime': ('Sources/L337Core/EquipmentRuntimeCatalog.swift',
               'ccf1ee2a940175f0d1a26a65aaa7b62ca7cf61d0eb33906f8b8feb6664210e27'),
    'purchases': ('Sources/L337Core/EquipmentTransactions.swift',
               '21a44066185ea17c8e3b262988c193cdef98bf88f80294d7a216767d93ad4f36'),
    'progression': ('Sources/L337Core/EquipmentPurchaseProgression.swift',
               '770618fcbb38330f568dbb862fc471762552cc17b55edfd407a1eb7462b0460c'),
    'commander': ('Sources/L337Core/CommanderEquipment.swift',
               'd5c9334210269abec7c3c4f2d16258a837b9dceb00dcec7184ffd92ae0ecec0c'),
    'mutations': ('Sources/L337Core/EquipmentMutations.swift',
               'a1d0bece73e7d6370f4cab8fc952d2e25a3d20dfceb89ce8fab189c4b65941d5'),
    'inventory': ('Sources/L337Core/OwnedEquipmentInventory.swift',
               '83009fdd488c170fff421cff398858acc77836d6668308ae69a743e8a1a7544f'),
    'interaction': ('Sources/L337Core/InventoryInteraction.swift',
               '7d6e3e89b256577c4516365863071159f96fafc70588bbd9c307ca9e4d72a6cc'),
    'performance': ('Sources/L337Core/ShipPerformance.swift',
               'b4647854a09f33a3c75f4e9218fdcade5f45eaa55c8b1014714fbfd183bd3d58'),
    'fleet': ('Sources/L337Core/OriginalFleetR3Data.swift',
               'ceba81e0702526ed3ff0e6e7f28868bae4ab508ae9ac6bfa9f18d63d78d7946a'),
    'fitting': ('Sources/L337Core/OriginalFleetR3Fitting.swift',
               '5a50e94cb510d7ce51da194cc3570159a28f1b7bd126e140a948192412d913ee'),
    'stations': ('Sources/L337Core/NativeStationServices.swift',
               'e74815c9dca25f05da4a8d9f5664c3492760b365c8f8b51cb6408a77acbe3559'),
    'projection': ('Sources/L337Core/ScriptEquipmentProjection.swift',
               'a1c3ffad9881c7d02f5a6cd31e6a51015b5c7735f43781f54850197e9313dac1'),
    'foundation': ('Sources/L337Core/EquipmentIssueTransactions.swift',
               '42653749b42fdb20af4577d41edc8527aff8c3990ac62ea331c44374850e88fa'),
    'accommodation': ('Sources/L337Core/ShipAccommodation.swift',
               '3866b7df7464409a1e12128605dc768c190190c0a821ee5f57f7c3694aaf3166'),
    'passengers': ('Sources/L337Core/PassengerCustody.swift',
               '9bb1f6239ce2b77c2bd6c7ce11f75a18d639c86ed5e74ad112be659cf6f88db1'),
    'drive': ('Sources/L337Core/HyperspaceDriveDefinition.swift',
               '931e8c50576e69dbaa41819b25e5a61c9ccdecefae9a581cb432ba76a89a91d3'),
    'shield': ('Sources/L337Core/ShieldBooster.swift',
               'a7a91bc9d94a4f8e805192752a0ee69e2e9f5fb4117b48941dc68ad4bc2ec528'),
}

# These source witnesses bind behavior omitted from the immutable numeric binary.
# They are not public constructors or runtime permissions.
WITNESSES = {
    'game': ['public var requiredTechnologyLevel: Int { technology + 1 }',
             'guard (checkedCargoTonnes.map { $0 <= capacity } ?? false) else { return false }'],
    'runtime': ['("Large Cargo Bay", .cargoExpansion)', '.init("utility-" + utility.rawValue)',
                'case .utility(let utility): slot = "utility." + utility.rawValue',
                'if utility == .cargoExpansion && performance.cargoExpansion == 0 { return nil }',
                'limits[node.id.family] = 1'],
    'purchases': ['boundHull: quote.location?.slot == .utility(.cargoExpansion) ? inventory.activeHull : nil',
                  'history.paid.contains(where: { $0.node == node })',
                  'inventory.items.contains(where: { $0.node == node })',
                  'items.append(item.moved(to: quote.location))',
                  'if !candidate.unlimitedCash { candidate.credits -= application.requiredDebit }'],
    'progression': ['guard snapshot.funds >= offer.price else {',
                    'throw EquipmentPurchaseError.replayedTransaction',
                    'policy: index == 0 && standard ? .standardFoundation : .shop,',
                    '("Navigation: Basic", "Basic Navigation Computer"),',
                    '("Basic Energy System", "Basic Energy System"),',
                    '("Basic Shield", "Basic Shield"),'],
    'commander': ['try OriginalFleetR3Fitting.validateUtilities(',
                  'standard: laser == .pulse && mount == .front)',
                  'for pylon in 0..<missiles { try append("Missile", at: .missilePylon(pylon)) }'],
    'mutations': ['guard candidate.isValid() else { throw EquipmentRuntimeError.incompatibleFit }',
                  'try inventory.store(expected: item, expectedRevision: expectedRevision, context: context)',
                  'guard docked, ownedEquipmentEventCapacityAvailable else {'],
    'inventory': ['boundHull: boundHull, preferredSlot: preferredSlot, retainedLegacyState: retainedLegacyState,',
                  'guard item.boundHull.map({ $0 == activeHull }) ?? true,',
                  'items[index] = item.moved(to: nil)',
                  'guard let index = items.firstIndex(where: { $0.id == expected.id }), items[index] == expected'],
    'interaction': ['![ComponentFittingSlot.shield, .energy, .navigation, .drive].contains {'],
    'performance': ['performance.cargo + (equipment.contains("Large Cargo Bay") ? performance.cargoExpansion : 0)'],
    'fitting': ['case .utility, .fuelStorage: return true',
                'guard ordinaryMountCount <= fitting.utilityMounts else {',
                'default: return fitting.equipmentUnits'],
    'stations': ['docked && dockedAtMainStation && commander.nativeStationStateValid',
                 '&& commander.nativeStations?.ledger.currentDock == nil',
                 'var stationAllowsEquipmentServices: Bool { stationAllowsMarketServices }'],
    'plugh': ['item.requiredTechnologyLevel <= currentSystem.technology',
              'return atPlugh ? Plugh.discount(price) : item.price'],
    'projection': ['if node.family == EquipmentRuntimeCatalog.laserFamily',
                   '|| node.family == EquipmentRuntimeCatalog.ammunitionFamily',
                   'EquipmentIssueTransactions.isIntrinsicFoundation(item) || isLegacyDockingItem(item)',
                   'return ShieldBooster.from(node.key) == nil ? 1 : 2'],
    'foundation': ['guard item.provenance == .standard,',
                   '$0.id == item.node && $0.policy == .standardFoundation',
                   'case (EquipmentPurchaseCatalog.shields, .shield),',
                   '(EquipmentPurchaseCatalog.energy, .energy),',
                   '(EquipmentPurchaseCatalog.navigation, .navigation),',
                   '(HyperspaceDriveDefinition.purchaseFamily, .drive):'],
    'accommodation': ['profile.equipmentUnits - otherFittedEquipmentUnits - cabins.count'],
    'passengers': ['return allowsXYZzyMaximumOutfit || passengerFit?.allowed == true'],
    'drive': ['case .basic: "Basic Hyperspace Drive"',
              'policy: tier == .basic ? .standardFoundation : .shop,'],
    'shield': ['public var equipmentName: String { label + " Shield Booster" }',
               'static func from(_ name: String) -> Self? { allCases.first { $0.equipmentName == name } }'],
}


def authenticate(identifier: str, raw: bytes) -> None:
    if identifier not in SOURCES or hashlib.sha256(raw).hexdigest() != SOURCES[identifier][1]:
        raise ValueError('Immutable equipment source hash mismatch: ' + identifier)


def read_blob(repository: Path, identifier: str) -> bytes:
    if identifier not in SOURCES:
        raise ValueError('Unselected equipment source')
    directory = Path(repository).resolve(strict=True)
    if not directory.is_dir():
        raise ValueError('Source repository must be a directory')
    raw = subprocess.check_output(
        ['git', '--no-pager', 'show', '--no-ext-diff', '--no-textconv', REVISION + ':' + SOURCES[identifier][0]],
        cwd=directory,
    )
    authenticate(identifier, raw)
    return raw


def integer(value: Any, maximum: int, label: str, minimum: int = 0) -> int:
    if type(value) is not int or not minimum <= value <= maximum:
        raise ValueError('Invalid ' + label)
    return value


def unique(rows: list[dict[str, Any]], field: str, value: Any) -> dict[str, Any]:
    selected = [row for row in rows if row[field] == value]
    if len(selected) != 1:
        raise ValueError('Missing/duplicate selected ' + field)
    return selected[0]


def witness_lines(raw: dict[str, bytes]) -> dict[str, list[dict[str, Any]]]:
    evidence = {}
    for key, witnesses in WITNESSES.items():
        text = raw[key].decode()
        evidence[key] = []
        for witness in witnesses:
            if witness not in text:
                raise ValueError('Missing equipment source witness: ' + key)
            evidence[key].append({'line': text[:text.index(witness)].count('\n') + 1, 'text': witness})
    return evidence


def selected_definition(authoring: dict[str, Any], game: str) -> dict[str, Any]:
    if authoring['version'] != 'l337-world-1':
        raise ValueError('Original equipment authoring version differs')
    item = unique(authoring['equipment'], 'id', 'cargo-bay')
    if set(item) != {'id', 'name', 'priceCR', 'technology'} or item['name'] != 'Large Cargo Bay':
        raise ValueError('Original cargo bay fields/identity differ')
    if type(item['priceCR']) not in (int, float):
        raise ValueError('Invalid original equipment price')
    tenths = Decimal(str(item['priceCR'])) * 10
    if not tenths.is_finite() or tenths <= 0 or tenths != tenths.to_integral_value():
        raise ValueError('Original equipment price is not exact credit tenths')
    price = integer(int(tenths), 2 ** 32 - 1, 'equipment price', minimum=1)
    technology = integer(item['technology'], 255, 'one-based equipment technology', minimum=1)
    matches = re.findall(r'\.init\(name: "Large Cargo Bay", price: (\d+), technology: (\d+)\)', game)
    if len(matches) != 1 or tuple(map(int, matches[0])) != (price, technology - 1):
        raise ValueError('Authoring/runtime equipment price or technology differs')
    return {'id': item['id'], 'name': item['name'], 'family': 'utility-cargoExpansion',
            'slot': 'utility.cargoExpansion', 'hull': 'SH025', 'priceTenthsCR': price,
            'requiredTechnologyLevel': technology}


def hull_fields(ship: dict[str, Any], r3: dict[str, Any], fleet: str) -> dict[str, int]:
    if (ship['id'], r3['id'], r3['schema']) != ('SH025', 'SH025', 'l337.fleet-design/2'):
        raise ValueError('Original equipment hull identity differs')
    if r3['fitting']['roleKitIncludedInBareHull'] is not False:
        raise ValueError('Original bare-hull role kit declaration differs')
    profile = unique(ship['profiles'], 'group', 'project-refit-v1')
    block = re.search(r'private static let sh025:.*?(?=private static let sh026:)', fleet, re.S)
    if block is None or SOURCES['r3'][1] not in block.group():
        raise ValueError('Missing authenticated SH025 runtime fitting')
    fields = {}
    selections = [
        ('baseHoldTonnes', ship['game']['cargo'], r3['performance']['cargoTonnes'], 'cargo', None),
        ('extraHoldTonnes', ship['game']['cargoExpansion'], r3['performance']['cargoExpansionTonnes'],
         'cargoExpansion', 'cargo_expansion_tonnes'),
        ('utilityMounts', r3['fitting']['utilityMounts'], r3['fitting']['utilityMounts'], 'utilityMounts', None),
        ('equipmentBudgetUnits', r3['fitting']['equipmentBudgetUnits'], r3['fitting']['equipmentBudgetUnits'],
         'equipmentUnits', 'equipment_capacity_units'),
    ]
    for name, value, original, swift, profile_key in selections:
        integer(value, 255 if name == 'utilityMounts' else 65535, name, minimum=1)
        numbers = re.findall(r'\b' + swift + r': (\d+)', block.group())
        if value != original or numbers != [str(value)]:
            raise ValueError('Original hull/runtime capacity differs: ' + name)
        if profile_key and unique(profile['fields'], 'key', profile_key)['number'] != value:
            raise ValueError('Original equipment profile differs: ' + name)
        fields[name] = value
    integer(fields['baseHoldTonnes'] + fields['extraHoldTonnes'], 65535, 'expanded hold')
    # The pinned projection counts this ordinary utility as one unit; R3 counts its actual utility slot once.
    return {**fields, 'itemUtilityMountCost': 1, 'itemEquipmentUnitCost': 1}


def selected_sites(worlds: dict[str, Any]) -> list[dict[str, Any]]:
    if worlds['version'] != 'l337-world-1' or len(worlds['systems']) != 2048:
        raise ValueError('Original equipment worlds differ')
    sites = []
    for system, name in [(7, 'Avenor'), (185, 'Renas')]:
        row = unique(worlds['systems'], 'key', f'G01-S{system:03}')
        integer(row['galaxy'], 0, 'site galaxy')
        integer(row['system'], 255, 'site system')
        if (row['system'], row['name']) != (system, name):
            raise ValueError('Original equipment station identity differs')
        technology = integer(row['technology'], 255, 'site technology', minimum=1)
        sites.append({'galaxy': 0, 'system': system, 'name': name, 'technology': technology})
    return sites


def binary(definition: dict[str, Any], sites: list[dict[str, Any]]) -> bytes:
    data = bytearray(struct.pack(
        '<8sIBHHBHBB', b'MEQP0001', definition['priceTenthsCR'], definition['requiredTechnologyLevel'],
        definition['baseHoldTonnes'], definition['extraHoldTonnes'], definition['utilityMounts'],
        definition['equipmentBudgetUnits'], definition['itemUtilityMountCost'], definition['itemEquipmentUnitCost']))
    for field in ['id', 'name', 'family', 'slot', 'hull']:
        raw = definition[field].encode()
        if not 1 <= len(raw) <= 255:
            raise ValueError('Invalid equipment text length')
        data.extend(bytes([len(raw)]))
        data.extend(raw)
    data.extend(bytes([len(sites)]))
    for site in sites:
        data.extend(struct.pack('<HB', site['system'], site['technology']))
    return bytes(data)


def convert(raw: dict[str, bytes]) -> dict[str, bytes]:
    if set(raw) != set(SOURCES):
        raise ValueError('Incomplete equipment source input set')
    for key, value in raw.items():
        authenticate(key, value)
    evidence = witness_lines(raw)
    definition = selected_definition(tomllib.loads(raw['authoring'].decode()), raw['game'].decode())
    definition.update(hull_fields(tomllib.loads(raw['ship'].decode()), tomllib.loads(raw['r3'].decode()),
                                  raw['fleet'].decode()))
    sites = selected_sites(json.loads(raw['worlds']))
    data = binary(definition, sites)
    manifest = {
        'sourceRevision': REVISION, 'sourceTree': TREE,
        'sources': {key: {'path': pin[0], 'sha256': pin[1], 'bytes': len(raw[key])} for key, pin in SOURCES.items()},
        'definition': definition, 'sites': sites, 'sourceWitnesses': evidence,
        'output': 'assets/legacy/sh025-cargo-bay.equipment',
        'outputBytes': len(data), 'outputSHA256': hashlib.sha256(data).hexdigest(),
        'format': 'MEQP0001 + LEu32 priceTenthsCR, u8 requiredTL, LEu16 base/extraHoldTonnes, u8 utilityMounts, LEu16 equipmentBudgetUnits, u8 itemUtilityMountCost/itemEquipmentUnitCost; five u8-length-prefixed UTF8 id/name/family/slot/hull; u8 siteCount then LEu16 system/u8 technology at galaxy0; no trailing bytes',
        'starterAdmission': {'source': 'Current edaa starter fit matched to original source standard/provenance/slot rules',
                             'items': ['Basic Shield', 'Navigation: Basic', 'Basic Energy System', 'Basic Hyperspace Drive',
                                       'Front Pulse Laser', 'Missile pylon0', 'Missile pylon1', 'Missile pylon2'],
                             'before': {'equipmentUnits': 0, 'utilityMounts': 0},
                             'withPaidFittedCargoBay': {'equipmentUnits': 1, 'utilityMounts': 1},
                             'limits': {'equipmentUnits': definition['equipmentBudgetUnits'],
                                        'utilityMounts': definition['utilityMounts']},
                             'authority': 'Reference calculation only; owner must inspect actual retained starter items and paid bay, not trust this projection'},
        'semantics': {'purchase': 'Actual ordinary main-station service and minimum TL4, authoritative28000 debit, unique paid item auto-fitted to active hull; refuse duplicate even when stored',
                      'fit': 'Exact actual paid hull-bound item at utility.cargoExpansion; storage retains item/provenance/boundHull; no repeated payment, refund or new device',
                      'capacity': '92t without fitted bay;127t with fitted bay; unfit refuses if actual occupied hold exceeds92t, including identified mission property',
                      'station': 'Source purchase requires actual main station; source fit/store requires docked; this joined slice supports the two original main orbitals only',
                      'nonAuthority': 'No role-kit grant, geometry change, crew, people consent, hull acquisition, free stock, migration or playable acceptance'},
        'origin': {'equipmentBudgetsAudit': 'assets/legacy/original-economy-origin.json',
                   'equipmentBudgetsAuditSHA256': 'a3a829192e7147ce5840924b1d0d8dbee419d8411f41057937ac26bdd1e557a8',
                   'hull': 'Pinned original project SH025 R3 and active runtime TOML agree on capacities; no geometry/material/font/bitmap copied or changed',
                   'preservedR3DecisionStatus': tomllib.loads(raw['r3'].decode())['decisionStatus'],
                   'scope': 'Original project-authored data and source-derived mechanics; historical generic labels retained; no third-party Elite/Oolite assets'},
        'notice': 'assets/legacy/Elite-L337-MIT.txt',
        'scope': 'One immutable equipment content component; no runtime/save/desktop implementation or acceptance',
    }
    return {'sh025-cargo-bay.equipment': data,
            'equipment-content-manifest.json': (json.dumps(manifest, indent=2) + '\n').encode()}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('repository', type=Path)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    outputs = convert({key: read_blob(args.repository, key) for key in SOURCES})
    for name, value in outputs.items():
        path = ROOT / 'assets/legacy' / name
        if args.check:
            if path.read_bytes() != value:
                raise ValueError('Committed equipment import mismatch: ' + name)
        else:
            path.write_bytes(value)
    print('Original SH025 cargo bay ' + ('matches' if args.check else 'imported'))


if __name__ == '__main__':
    main()
