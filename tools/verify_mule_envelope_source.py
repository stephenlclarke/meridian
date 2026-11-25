"""Verify source bytes, old-source preservation and current-gameplay adaptation only."""
import ast
import hashlib
import json
from pathlib import Path
import re
import struct
import tomllib

if not __debug__:
    raise RuntimeError("Mule source verification requires Python assertions")

RUST_TEST_NAME = r'#\[test\]\s*fn\s+(\w+)\('
FLIGHT_SOURCE = 'flight.rs'
FLIGHT_PERSISTENCE = 'flight/persistence.rs'
EMP_TESTS = 'flight/emp/tests.rs'
LIB_SOURCE = 'lib.rs'
ROOT = Path(__file__).resolve().parents[1]
PROTECTED_CORE = Path('integration/sh001-mule/PROTECTED-CORE.json')
FLIGHT_COMPUTER_INVERSE = Path('integration/sh001-mule/FLIGHT-COMPUTER-INVERSE.json')
FLIGHT_COMPUTER_BASE = 'fcec61d182b66478846b92390ad17805b0dfc7e0'
FLIGHT_COMPUTER_INVERSE_SHA256 = '094fb89c24d15d3b882c49e24a149a5ccb188ed6e949577691440f76b3872a1d'


def sha(data):
    return hashlib.sha256(data).hexdigest()


def protected_core_pins(root):
    pin_bytes = (root / PROTECTED_CORE).read_bytes()
    assert sha(pin_bytes) == '8a2929218820a8e15536aa554c0f8df56a23981944be85ec1b1947778fd48133'
    baseline = json.loads(pin_bytes)
    assert baseline['base'] == '792f4485372ae37d0c1d583ae11229e85932ce2f'
    return baseline['sha256']


def flight_computer_inverse(root, protected):
    manifest = (root / FLIGHT_COMPUTER_INVERSE).read_bytes()
    assert sha(manifest) == FLIGHT_COMPUTER_INVERSE_SHA256, 'flight computer inverse manifest'
    inverse = json.loads(manifest)
    assert inverse['base'] == FLIGHT_COMPUTER_BASE, 'flight computer inverse base'
    files = inverse['files']
    assert files and files.keys() <= protected.keys(), 'flight computer inverse paths'
    return files


def undo_leopard_fighter_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod leopard_fighter;\n'
    before, after = b'pub mod lanner;\n', b'pub mod llama;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_hippo_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod hippo;\n'
    before, after = b'pub mod harpy_gunship;\n', b'pub mod hobby_fighter;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_magpie_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod magpie;\n'
    before, after = b'pub mod llama;\n', b'pub mod mining;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_tapir_mk_ii_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod tapir_mk_ii;\n'
    before, after = b'pub mod tamaraw;\n', b'pub mod tapir_shuttle;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_tapir_shuttle_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod tapir_shuttle;\n'
    before, after = b'pub mod tamaraw;\n', b'pub mod traffic;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_wildebeest_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod wildebeest;\n'
    before, after = b'pub mod vulture_raider;\n', b'pub mod wisent;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_cavy_racer_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod cavy_racer;\n'
    before, after = b'pub mod caracal_scout;\n', b'pub mod celestial;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_ox_mk_ii_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod ox_mk_ii;\n'
    before, after = b'pub mod ox;\n', b'pub mod rabbit_explorer;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_ox_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod ox;\n'
    before, after = b'pub mod oryx_voyager;\n', b'pub mod rabbit_explorer;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_anoa_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod anoa;\n'
    before, after = b'pub mod alpaca;\n', b'pub mod antelope;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_wisent_mk_ii_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod wisent_mk_ii;\n'
    before, after = b'pub mod wisent;\n', b'pub mod wombat;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_wisent_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod wisent;\n'
    before, after = b'pub mod vulture_raider;\n', b'pub mod wombat;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_shikra_interdictor_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod shikra_interdictor;\n'
    before, after = b'pub mod shikra;\n', b'pub mod springbok_starliner;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_shikra_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod shikra;\n'
    before, after = b'pub mod seriema_courier;\n', b'pub mod springbok_starliner;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_giraffe_liner_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod giraffe_liner;\n'
    before, after = b'pub mod freight;\n', b'pub mod gopher;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_aurochs_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod aurochs;\n'
    before, after = b'pub mod arna_cutter;\n', b'pub mod badger;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_elephant_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod elephant;\n'
    before, after = b'pub mod eagle_cruiser;\n', b'pub mod elk;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_gopher_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod gopher;\n'
    before, after = b'pub mod freight;\n', b'pub mod goshawk;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_antelope_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod antelope;\n'
    before, after = b'pub mod alpaca;\n', b'pub mod aplomado_interceptor;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_springbok_starliner_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod springbok_starliner;\n'
    before, after = b'pub mod seriema_courier;\n', b'pub mod station;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_elk_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod elk;\n'
    before, after = b'pub mod eagle_cruiser;\n', b'pub mod equipment;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_alpaca_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod alpaca;\n'
    before, after = b'#![forbid(unsafe_code)]\n\n', b'pub mod aplomado_interceptor;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_laggar_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod laggar;\n'
    before, after = b'pub mod jaguar;\n', b'pub mod lanner;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_llama_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod llama;\n'
    before, after = b'pub mod lanner;\n', b'pub mod mining;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_hare_explorer_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod hare_explorer;\n'
    before, after = b'pub mod griffon_spectre;\n', b'pub mod harpy_assault;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_jaguar_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod jaguar;\n'
    before, after = b'pub mod jackal;\n', b'pub mod lanner;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_carabao_highliner_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod carabao_highliner;\n'
    before, after = b'pub mod carabao_freighter;\n', b'pub mod caracal_scout;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_carabao_freighter_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod carabao_freighter;\n'
    before, after = b'pub mod baza_mk_ii;\n', b'pub mod caracal_scout;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_oryx_voyager_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod oryx_voyager;\n'
    before, after = b'pub mod navigation;\n', b'pub mod rabbit_explorer;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_tamaraw_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod tamaraw;\n'
    before, after = b'pub mod station_services;\n', b'pub mod traffic;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_hog_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod hog;\n'
    before, after = b'pub mod hobby_fighter;\n', b'pub mod hyena;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_dzo_mk_ii_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod dzo_mk_ii;\n'
    before, after = b'pub mod dzo;\n', b'pub mod eagle_cruiser;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_dzo_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod dzo;\n'
    before, after = b'pub mod docking;\n', b'pub mod eagle_cruiser;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_griffon_spectre_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod griffon_spectre;\n'
    before, after = b'pub mod griffon_mk_ii;\n', b'pub mod harpy_assault;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_griffon_mk_ii_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod griffon_mk_ii;\n'
    before, after = b'pub mod griffon;\n', b'pub mod harpy_assault;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_griffon_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod griffon;\n'
    before, after = b'pub mod goshawk_custom;\n', b'pub mod harpy_assault;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_baza_mk_ii_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod baza_mk_ii;\n'
    before, after = b'pub mod baza_fighter;\n', b'pub mod caracal_scout;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_baza_fighter_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod baza_fighter;\n'
    before, after = b'pub mod barbary;\n', b'pub mod caracal_scout;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_badger_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod badger;\n'
    before, after = b'pub mod arna_cutter;\n', b'pub mod banteng;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_rhinoceros_trader_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod rhinoceros_trader;\n'
    before, after = b'pub mod rabbit_explorer;\n', b'pub mod saiga;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_rabbit_explorer_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod rabbit_explorer;\n'
    before, after = b'pub mod navigation;\n', b'pub mod saiga;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_vulture_raider_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod vulture_raider;\n'
    before, after = b'pub mod traffic;\n', b'pub mod wombat;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_arna_cutter_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod arna_cutter;\n'
    before, after = b'pub mod aplomado_interceptor;\n', b'pub mod banteng;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_seriema_courier_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod seriema_courier;\n'
    before, after = b'pub mod save;\n', b'pub mod station;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_moose_clipper_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod moose_clipper;\n'
    before, after = b'pub mod mining;\n', b'pub mod mule;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_saiga_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod saiga;\n'
    before, after = b'pub mod navigation;\n', b'pub mod saker;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_wombat_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod wombat;\n'
    before, after = b'pub mod traffic;\n', b'pub mod yak;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_barbary_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod barbary;\n'
    before, after = b'pub mod banteng;\n', b'pub mod caracal_scout;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_muskox_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod muskox;\n'
    before, after = b'pub mod mule;\n', b'pub mod navigation;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_falconet_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod falconet;\n'
    before, after = b'pub mod erne_corvette;\n', b'mod flight;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_aplomado_interceptor_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod aplomado_interceptor;\n'
    before, after = b'#![forbid(unsafe_code)]\n\n', b'pub mod banteng;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_condor_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod condor;\n'
    before, after = b'pub mod celestial;\n', b'pub mod docking;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_hobby_fighter_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod hobby_fighter;\n'
    before, after = b'pub mod harpy_gunship;\n', b'pub mod hyena;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_caracal_scout_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod caracal_scout;\n'
    before, after = b'pub mod banteng;\n', b'pub mod celestial;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_banteng_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod banteng;\n'
    before, after = b'#![forbid(unsafe_code)]\n\n', b'pub mod celestial;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_jackal_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod jackal;\n'
    before, after = b'pub mod hyena;\n', b'pub mod lanner;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_yak_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod yak;\n'
    before, after = b'pub mod traffic;\n', b'\npub use flight::GoshawkCustomApproachSnapshot;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_hyena_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod hyena;\n'
    before, after = b'pub mod harpy_gunship;\n', b'pub mod lanner;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_lanner_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod lanner;\n'
    before, after = b'pub mod harpy_gunship;\n', b'pub mod mining;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_gunship_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod harpy_gunship;\n'
    before, after = b'pub mod harpy_dropship;\n', b'pub mod mining;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_dropship_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod harpy_dropship;\n'
    before, after = b'pub mod harpy_assault;\n', b'pub mod mining;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_erne_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod erne_corvette;\n'
    before, after = b'pub mod equipment;\n', b'mod flight;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_harpy_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod harpy_assault;\n'
    before, after = b'pub mod goshawk_custom;\n', b'pub mod mining;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_eagle_publication(relative, data):
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod eagle_cruiser;\n'
    before, after = b'pub mod docking;\n', b'pub mod equipment;\n'
    span = before + declaration + after
    assert data.count(declaration) == 1 and data.count(span) == 1, relative
    return data.replace(span, before + after, 1)


def undo_saker_publication(relative, data):
    """Restore the reviewed lib.rs insertion before checking the Flight Computer hashes."""
    if relative != LIB_SOURCE:
        return data
    declaration = b'pub mod saker;\n'
    anchor = b'pub mod navigation;\n'
    assert data.count(declaration) == 1 and data.count(anchor + declaration) == 1, relative
    return data.replace(anchor + declaration, anchor, 1)


def undo_flight_computer(relative, data, inverse):
    """Restore exact pre-slice bytes, without fuzzy matching or exclusions."""
    if relative not in inverse:
        return data
    record = inverse[relative]
    assert sha(data) == record['afterSHA256'], relative
    assert record['hunks'], relative
    restored = bytearray()
    cursor = 0
    for hunk in record['hunks']:
        start = hunk['offset']
        current = hunk['after'].encode('utf-8')
        previous = hunk['before'].encode('utf-8')
        assert cursor <= start <= len(data) and current != previous, relative
        end = start + len(current)
        assert end <= len(data) and data[start:end] == current, relative
        restored.extend(data[cursor:start])
        restored.extend(previous)
        cursor = end
    restored.extend(data[cursor:])
    result = bytes(restored)
    assert sha(result) == record['beforeSHA256'], relative
    return result


def undo_commander_launch(relative, data):
    replacements = {
        FLIGHT_SOURCE: [
            (
                b'        self.launch(expected_tick, false)\n'
                b'    }\n\n'
                b'    /// Resume and depart from the current commander berth as one World transaction.\n'
                b'    /// A refused departure also preserves the original pause and review sequence.\n'
                b'    pub fn request_launch_from_commander(&mut self, expected_tick: u64) -> Result<(), StepError> {\n'
                b'        self.launch(expected_tick, true)\n'
                b'    }\n\n'
                b'    fn launch(&mut self, expected_tick: u64, from_commander: bool) -> Result<(), StepError> {\n',
                b'',
            ),
            (b'        if self.is_paused() && !from_commander {\n', b'        if self.is_paused() {\n'),
            (b'            || (from_commander && self.current_dock_context().is_none())\n', b''),
            (b'            .map(mission::FreightSession::launch_epoch)\n',
             b'            .map(|session| session.next_epoch())\n'),
            (b'            session.publish_launch(epoch);\n', b'            session.rebind_dock(None, epoch);\n'),
        ],
        'flight/mission.rs': [
            (
                b'    pub(super) fn launch_epoch(&self) -> Result<u64, MissionError> {\n'
                b'        // Resuming and departing each invalidate the prior review authority.\n'
                b'        self.epoch\n'
                b'            .checked_add(1 + u64::from(self.paused))\n'
                b'            .ok_or(MissionError::SequenceExhausted)\n'
                b'    }\n'
                b'    pub(super) fn publish_launch(&mut self, epoch: u64) {\n'
                b'        self.paused = false;\n'
                b'        self.rebind_dock(None, epoch);\n'
                b'    }\n',
                b'',
            ),
        ],
    }
    for current, original in replacements.get(relative, []):
        assert data.count(current) == 1, relative
        data = data.replace(current, original, 1)
    return data


def verify_inputs(folder):
    manifest = json.loads((folder / 'MANIFEST.json').read_text())
    for name, pin in manifest['files'].items():
        data = (folder / name).read_bytes()
        assert len(data) == pin['bytes'] and sha(data) == pin['sha256'], name
    tree = ast.parse((ROOT / 'tools/import_mule_envelope_source.py').read_text())
    pins = next(ast.literal_eval(node.value) for node in tree.body if isinstance(node, ast.Assign) and any(isinstance(target, ast.Name) and target.id == 'PINS' for target in node.targets))
    for name, expected in pins.items():
        assert sha((folder / name).read_bytes()) == expected, name


def verify_component(model, part, assigned, faces, well_pieces):
    for key in ['vertexStart', 'vertexCount', 'faceStart', 'faceCount']:
        assert part[key] == assigned[key]
    assert part['index'] == assigned['component'] and part['name'] == assigned['name']
    name = part['name']
    piece = assigned['piece']
    if name.startswith('body primary pressure envelope/service-well-'):
        key = (model, name.split('/', 2)[1])
        assert piece == well_pieces.setdefault(key, 16 + len(well_pieces))
    elif name == 'body primary pressure envelope/longitudinal-service-strut-0':
        assert piece == 32
    elif name == 'body primary pressure envelope/longitudinal-service-strut-1':
        assert piece == 33
    elif name == 'body primary pressure envelope':
        assert piece == 0
    else:
        assert 1 <= piece < 16
    first_vertex = part['vertexStart']
    end_vertex = first_vertex + part['vertexCount']
    first_face = part['faceStart']
    end_face = first_face + part['faceCount']
    for face in faces[first_face:end_face]:
        assert len(face) == 3 and all(first_vertex <= i < end_vertex for i in face)


def verify_geometry(folder):
    resource = tomllib.loads((folder / 'resource.toml').read_text())
    original = tomllib.loads((folder / 'original-design.toml').read_text())
    owner = json.loads((folder / 'ownership.json').read_text())
    groups = json.loads((folder / 'COMPONENT-PIECES.json').read_text())
    vertices = bytearray(b'MSH001V1') + struct.pack('<II', 4360, 984)
    triangles = bytearray()
    membership = bytearray()
    well_pieces = {}
    for model, offset, counts in [('design', 0, (4360, 7236, 371)), ('nativeDesign', 4360, (984, 1772, 49))]:
        assert resource[model] == original[model]
        assert (len(resource[model]['vertices']), len(resource[model]['faces']), len(owner['models'][model]['parts'])) == counts
        assert len(groups[model]) == counts[2]
        for part, assigned in zip(owner['models'][model]['parts'], groups[model], strict=True):
            verify_component(model, part, assigned, resource[model]['faces'], well_pieces)
            membership.extend(bytes([assigned['piece']]) * assigned['vertexCount'])
        for point in resource[model]['vertices']:
            vertices.extend(struct.pack('<ddd', *point))
        for face in resource[model]['faces']:
            triangles.extend(struct.pack('<III', *(index + offset for index in face)))
    assert bytes(vertices) == (folder / 'all-source.vertices').read_bytes()
    assert bytes(triangles) == (folder / 'source-triangles.bin').read_bytes()
    assert bytes(membership) == (folder / 'source-piece-membership.bin').read_bytes()
    assert len(well_pieces) == 16
    assert set(membership) == set(range(34))
    return groups


def undo_renas_encounter(relative, data):
    # Restore only the reviewed Renas additions before checking the protected blobs.
    replacements = {
        FLIGHT_SOURCE: [
            (b'mod mule_renas;\n', b''),
            (b'    mule_renas: Option<crate::traffic::mule_validation::State>,\n', b''),
            (b'            mule_renas: None,\n', b''),
            (b'        let mule_renas = self\n            .prepare_renas_mule(next.mode, navigation)\n            .map_err(|_| StepError::UnsafeLaunch)?;\n        if let Some(actor) = &mule_renas {\n            self.validate_renas_mule_launch(actor, launch_pose)?;\n        }\n', b''),
            (b'        self.mule_renas = mule_renas;\n', b''),
            (b'        self.refresh_deferred_renas_mule();\n', b''),
            (b'            if !self.traffic_and_renas_mule_motion_clear(before, after, angular_travel) {\n', b'            if !self.traffic_motion_clear(before, after) {\n'),
        ],
        FLIGHT_PERSISTENCE: [
            (b'            mule_renas: None,\n', b''),
            (b'        candidate\n            .restore_renas_mule()\n            .map_err(|_| SaveError::ContentMismatch)?;\n', b''),
        ],
        'flight/mission.rs': [
            (b'    fn renas_mule_is_the_same_actor_on_the_first_completed_freight_sortie_after_continue() {\n        let mut world = World::new_commander(501).unwrap();\n', b'    fn public_contact_back_pause_tick_and_owner_changes_do_not_manufacture_consent() {\n        let mut world = World::new_commander(51).unwrap();\n', '02ae2146927ff0e3d37b1c4605c72614c585399f2a46e1134c101e7c48183c8e'),
        ],
        EMP_TESTS: [
            (b'        mule_renas: None,\n', b''),
        ],
        'freight/g02.rs': [
            (b"    /// The original G02 transaction's validated Renas main orbital berth.\n    pub(crate) fn renas_departure_binding(self) -> Option<(u64, u64, u64, u64)> {\n        (self.dock.system == RENAS\n            && self.dock.kind == StationKind::MainOrbital\n            && self.dock.station_actor.entity == 1)\n            .then_some((\n                self.dock.owner,\n                self.dock.hull,\n                self.dock.arrival_sequence,\n                self.dock.station_actor.generation,\n            ))\n    }\n\n", b''),
        ],
    }
    for rule in replacements.get(relative, []):
        if len(rule) == 2:
            current, previous = rule
            assert data.count(current) == 1, relative
            data = data.replace(current, previous, 1)
        else:
            first, after, expected = rule
            assert data.count(first) == data.count(after) == 1, relative
            start = data.index(first)
            end = data.index(after, start + len(first))
            assert sha(data[start:end]) == expected, relative
            data = data[:start] + data[end:]
    return data


def undo_goshawk_world(relative, data):
    # Restore the preceding SH001 dispatch and verify the same protected blobs.
    replacements = {
        'flight.rs': [
            (b'mod goshawk_validation;\n', b''),
            (b'pub use goshawk_validation::GoshawkCustomApproachSnapshot;\n', b''),
            (b'pub use goshawk_validation::{GoshawkApproachSnapshot, GoshawkApproachStatus};\n', b''),
            (b'    goshawk_validation: Option<goshawk_validation::Approach>,\n', b''),
            (b'            goshawk_validation: None,\n', b''),
            (b'        if let Some(result) = self.controlled_hull_step(input) {\n            return result;\n        }\n', b'        if self.mule_validation.is_some() {\n            return self.step_mule_validation(input);\n        }\n'),
        ],
        FLIGHT_PERSISTENCE: [(b'            goshawk_validation: None,\n', b'')],
        EMP_TESTS: [(b'        goshawk_validation: None,\n', b'')],
        LIB_SOURCE: [
            (b'pub mod goshawk_custom;\n', b''),
            (b'pub use flight::GoshawkCustomApproachSnapshot;\n', b''),
            (b'pub use flight::{GoshawkApproachSnapshot, GoshawkApproachStatus};\n', b''),
        ],
    }
    for current, previous in replacements.get(relative, []):
        assert data.count(current) == 1, relative
        data = data.replace(current, previous, 1)
    return data


def verify_protected_core(root):
    # Undo the reviewed Mule hooks and the equivalent finite-speed range check.
    # Checking every pre-existing core blob catches unrelated gameplay edits.
    additions = {
        FLIGHT_SOURCE: [
            b'mod mule_validation;\n',
            b'    mule_validation: Option<crate::traffic::mule_validation::State>,\n',
            b'            mule_validation: None,\n',
            b'        if self.mule_validation.is_some() {\n            return self.step_mule_validation(input);\n        }\n',
        ],
        EMP_TESTS: [b'        mule_validation: None,\n'],
        FLIGHT_PERSISTENCE: [b'            mule_validation: None,\n'],
        LIB_SOURCE: [b'pub mod mule;\n', b'pub mod goshawk;\n'],
        'station.rs': [b'mod mule;\n'],
        'traffic.rs': [
            b'pub(crate) mod mule_validation;\n',
            b'pub use mule_validation::{MuleActorSnapshot, MuleValidationRefusal};\n',
        ],
    }
    baseline = protected_core_pins(root)
    inverse = flight_computer_inverse(root, baseline)
    checked = set()
    unchanged = 0
    for relative, expected in baseline.items():
        current = (root / 'crates/meridian-core/src' / relative).read_bytes()
        data = undo_leopard_fighter_publication(relative, current)
        data = undo_hippo_publication(relative, data)
        data = undo_magpie_publication(relative, data)
        data = undo_tapir_mk_ii_publication(relative, data)
        data = undo_tapir_shuttle_publication(relative, data)
        data = undo_wildebeest_publication(relative, data)
        data = undo_cavy_racer_publication(relative, data)
        data = undo_ox_mk_ii_publication(relative, data)
        data = undo_ox_publication(relative, data)
        data = undo_anoa_publication(relative, data)
        data = undo_wisent_mk_ii_publication(relative, data)
        data = undo_wisent_publication(relative, data)
        data = undo_shikra_interdictor_publication(relative, data)
        data = undo_shikra_publication(relative, data)
        data = undo_giraffe_liner_publication(relative, data)
        data = undo_aurochs_publication(relative, data)
        data = undo_elephant_publication(relative, data)
        data = undo_gopher_publication(relative, data)
        data = undo_antelope_publication(relative, data)
        data = undo_springbok_starliner_publication(relative, data)
        data = undo_elk_publication(relative, data)
        data = undo_alpaca_publication(relative, data)
        data = undo_laggar_publication(relative, data)
        data = undo_llama_publication(relative, data)
        data = undo_hare_explorer_publication(relative, data)
        data = undo_jaguar_publication(relative, data)
        data = undo_carabao_highliner_publication(relative, data)
        data = undo_carabao_freighter_publication(relative, data)
        data = undo_oryx_voyager_publication(relative, data)
        data = undo_tamaraw_publication(relative, data)
        data = undo_hog_publication(relative, data)
        data = undo_dzo_mk_ii_publication(relative, data)
        data = undo_dzo_publication(relative, data)
        data = undo_griffon_spectre_publication(relative, data)
        data = undo_griffon_mk_ii_publication(relative, data)
        data = undo_griffon_publication(relative, data)
        data = undo_baza_mk_ii_publication(relative, data)
        data = undo_baza_fighter_publication(relative, data)
        data = undo_badger_publication(relative, data)
        data = undo_rhinoceros_trader_publication(relative, data)
        data = undo_rabbit_explorer_publication(relative, data)
        data = undo_vulture_raider_publication(relative, data)
        data = undo_arna_cutter_publication(relative, data)
        data = undo_seriema_courier_publication(relative, data)
        data = undo_moose_clipper_publication(relative, data)
        data = undo_saiga_publication(relative, data)
        data = undo_wombat_publication(relative, data)
        data = undo_barbary_publication(relative, data)
        data = undo_muskox_publication(relative, data)
        data = undo_falconet_publication(relative, data)
        data = undo_aplomado_interceptor_publication(relative, data)
        data = undo_condor_publication(relative, data)
        data = undo_hobby_fighter_publication(relative, data)
        data = undo_caracal_scout_publication(relative, data)
        data = undo_banteng_publication(relative, data)
        data = undo_jackal_publication(relative, data)
        data = undo_yak_publication(relative, data)
        data = undo_hyena_publication(relative, data)
        data = undo_lanner_publication(relative, data)
        data = undo_gunship_publication(relative, data)
        data = undo_dropship_publication(relative, data)
        data = undo_erne_publication(relative, data)
        data = undo_harpy_publication(relative, data)
        data = undo_eagle_publication(relative, data)
        data = undo_saker_publication(relative, data)
        data = undo_flight_computer(relative, data, inverse)
        data = undo_goshawk_world(relative, data)
        data = undo_renas_encounter(relative, data)
        for addition in additions.get(relative, []):
            assert data.count(addition) == 1, relative
            data = data.replace(addition, b'', 1)
        data = undo_commander_launch(relative, data)
        if relative == LIB_SOURCE:
            publication = b'pub mod stag_liner;\n\n'
            assert data.count(publication) == 1, relative
            data = data.replace(publication, b'', 1)
        if relative == FLIGHT_SOURCE:
            speed_line = b'        if !(0.0..=MAX_SPEED).contains(&self.state.speed) {\n'
            original_line = b'        if !self.state.speed.is_finite() || !(0.0..=MAX_SPEED).contains(&self.state.speed) {\n'
            assert data.count(speed_line) == 1, relative
            data = data.replace(speed_line, original_line, 1)
        assert sha(data) == expected, relative
        checked.add(relative)
        unchanged += data == current
    assert set(additions) <= checked and len(checked) == 115
    return len(checked), len(additions), unchanged, len(inverse)


def verify_test_registry(criteria):
    files = {
        'newCoreTests': ('mule/tests.rs', 11),
        'worldCoreTests': ('flight/mule_validation/tests.rs', 6),
        'tightnessCoreTests': ('mule/tightness_tests.rs', 5),
        'geometryCoreTests': ('mule/geometry.rs', 1),
        'restoreCoreTests': ('flight/mule_renas/tests.rs', 1),
    }
    counts = {}
    for group, (module, expected) in files.items():
        path = Path('crates/meridian-core/src') / module
        names = re.findall(RUST_TEST_NAME, (ROOT / path).read_text())
        entries = criteria[group]
        assert len(names) == expected and names == [entry['name'] for entry in entries], group
        assert all(entry['file'] == path.as_posix() and entry['status'] == 'UNRUN' for entry in entries), group
        counts[group] = len(names)
    total = sum(counts.values())
    assert total == criteria['totalMuleCoreTests']
    return {**counts, 'totalMuleCoreTests': total}


def verify_historical_bindings(folder, groups):
    # Preserve the historical component and World patch identities separately.
    binding = json.loads((ROOT / 'integration/sh001-mule/CURRENT-GAMEPLAY.json').read_text())
    patch = (ROOT / 'integration/sh001-mule/CURRENT-GAMEPLAY.patch').read_bytes()
    assert sha(patch) == binding['moduleDeclarationPatchSHA256']
    plus = [line for line in patch.decode().splitlines() if line.startswith('+') and not line.startswith('+++')]
    assert plus == ['+pub mod mule;', '+mod mule;']
    criteria = json.loads((ROOT / 'integration/sh001-mule/CRITERIA.json').read_text())
    speed_contract = tomllib.loads((folder / 'contract.toml').read_text())['performance']['maxSpeed']
    speed_source = (ROOT / 'crates/meridian-core/src/mule.rs').read_text()
    speed_constant = re.search(r'const MULE_MAX_SPEED_UNITS: f32 = ([0-9.]+);', speed_source)
    assert speed_constant and float(speed_constant[1]) == speed_contract
    tightness = (ROOT / 'crates/meridian-core/src/mule/tightness_tests.rs').read_text()
    rim_text = tightness.split('const WELL_RIMS:', 1)[1].split('];', 1)[0]
    rim_indices = [(int(piece), int(first)) for piece, first in re.findall(r'\((\d+), (\d+)\)', rim_text)]
    assert rim_indices == [(part['piece'], part['vertexStart']) for part in groups['design'] if '/service-well-' in part['name'] and '/recess-' not in part['name']]
    assert len(rim_indices) == 84
    world_binding = json.loads((ROOT / 'integration/sh001-mule/WORLD-BINDING.json').read_text())
    world_patch = (ROOT / world_binding['canonicalPatch']['path']).read_bytes()
    assert len(world_patch) == world_binding['canonicalPatch']['bytes']
    assert sha(world_patch) == world_binding['canonicalPatch']['sha256']
    assert len(world_binding['currentRootCoreFiles']) == 135
    assert len(world_binding['modifiedCurrentFiles']) == 3
    fixture = criteria['prePrRenasRestoreFixture']
    fixture_bytes = (ROOT / fixture['path']).read_bytes()
    assert len(fixture_bytes) == fixture['bytes']
    assert fixture_bytes[:10] == b'MERSAVE1\x11\x00'
    assert sha(fixture_bytes) == fixture['sha256']
    return verify_test_registry(criteria)


def verify():
    folder = ROOT / 'assets/fleet/sh001'
    verify_inputs(folder)
    groups = verify_geometry(folder)
    checked, hooks, unchanged, flight_computer = verify_protected_core(ROOT)
    counts = verify_historical_bindings(folder, groups)
    return {
        'status': 'PASS_SOURCE_ONLY',
        'protectedGameplayCoreFilesUnchanged': unchanged,
        'protectedCoreFilesWithCommanderLaunch': 2,
        'protectedCoreFilesWithMuleHooks': hooks,
        'protectedCoreFilesWithFlightComputerChanges': flight_computer,
        'protectedCoreFilesChecked': checked,
        **counts,
        'vertexEncoding': 5344,
        'triangleEncoding': 9008,
        'constructorsOrContactOrContainmentOrCompilerExecuted': False,
    }


if __name__ == '__main__':
    print(json.dumps(verify()))
