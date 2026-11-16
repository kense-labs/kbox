# Plan036

Trace clear/build/template publication/restore/drain/consumer lifetimes, establish
an actual unowned-directory deletion with an empty pool in a dedicated child home.
Choose minimal typed ownership using existing cap-std/tempfile adapters and actual
VM handles. Independent attempts must never overwrite the same template files;
cleanup acts only on remembered owned directory identity after consumers finish.
Keep original TemplateState control logic as one state machine, original errors,
routes and generation authority. Update tests/fixtures around explicit ownership
without lowering deletion/data-preservation requirements. Then owner checks,
source-bound rebuild/install, native Mac pool/caller canaries, complete audit and
Git closeout; do not turn missing Linux snapshot hardware into a global stop.
