# Brand zero residue (user R5)

Date: 2026-11-17  
Constraint: code and docs must not retain the forbidden upstream product brand.

## Gate

```bash
python3 scripts/check-sandbox-brand.py
```

Needle assembled at runtime (`"ki"+"ri"` / `"run"+brand`) so the gate source
stays clean. Whole-word match; does not flag Linux `HISI_KIRIN`.

## Result

```
brand scan: clean (zero product-brand residue)
```

Also rewritten: `GOALS.md`, Spec050 artifacts, `PROVENANCE.md`, NOTICE headers,
evidence notes. Provenance cites commit `2d6c2d6` without brand wording.

## Explicitly not claimed

- Product Pass
- Default sandbox already runnable
