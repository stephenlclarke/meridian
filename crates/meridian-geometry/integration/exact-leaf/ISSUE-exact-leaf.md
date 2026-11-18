# Issue: box support admits empty outward triangle sections

## Motivation and required behavior

The qualified original-five still refuses every constructor on operations. Oblique leaf boxes can meet an outward plane while the exact triangle is entirely on one side. The cache already retains exact active-plane vertex distances but the old leaf query does not use them.

## Correction and risks

Use exact positive/nonpositive triangle support at leaves, preserving independent physical flags, node/owner/source-order logic, fields and contacts. Retain complete parent controls. See [the contract](CONTRACT.md) for proof, complete work/storage charges and three authored tests. Current runtime behavior and benefit are unproven; extra checks/scratch may cause earlier refusal. Tracking uses the task slug; no GitHub issue number is assigned.
