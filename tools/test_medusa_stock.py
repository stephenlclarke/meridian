"""Regressions for Medusa stock dry source and its read-only admission."""

import json
import unittest
from unittest.mock import patch

import prepare_medusa_stock as stock


class MedusaStockTests(unittest.TestCase):
    def test_rust_constant_matches_exact_source(self):
        self.assertEqual(stock.RUST.read_bytes(), stock.rust_source())
        model = stock.reference()
        dry = model["emptyDryReference"]
        self.assertEqual(dry["massKg"], 40712)
        tensor = dry["inertiaAboutCentreOfMassKgM2"]
        self.assertEqual([tensor[i] for i in (1, 2, 5)], [tensor[i] for i in (3, 6, 7)])
        self.assertEqual(len(dry["centreMetres"]), 3)

    def test_configuration_stays_stock_and_has_no_acquisition_authority(self):
        model = stock.reference()
        self.assertTrue(model["nativeAdmissionBlocked"])
        self.assertFalse(model["occupiedPeopleOrPaidEquipmentGranted"])
        self.assertFalse(model["operatingAuthorityGranted"])
        self.assertEqual(model["preservedR3"]["accommodation"]["playerConversionCrew"], 1)
        classes = json.loads((stock.PROOF / "MEDUSA-FLAGSHIP.json").read_bytes())["configurations"]
        self.assertEqual(classes["catalogueBase"]["restoredStockValues"]["energy"], 410)
        self.assertEqual(classes["catalogueBase"]["restoredStockValues"]["shield"], 300)
        self.assertFalse(classes["missionInstance"]["fullyUpgraded"])
        self.assertFalse(classes["plughShowroom"]["freeMaximumFit"])

    def test_changed_bytes_refuse_before_generating_stock(self):
        raw = stock.proof_bytes

        def changed(path, expected):
            return raw(path, "0" * 64 if path.name == "FINITE-STOCK.json" else expected)

        with patch.object(stock, "proof_bytes", side_effect=changed):
            with self.assertRaisesRegex(ValueError, "Changed Medusa stock source"):
                stock.rust_source()
