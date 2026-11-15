# Retained failures

- memory-before1: actual0passed/2failed/0ignored on the existing Mac admission
  and process completion paths. Fixed the three missing resource mappings once
  in the shared helper; memory-after1 actual29passed/0failed/4ignored.
- Static probe cross-build1 is preparation, not native evidence. Its original
  source/binary are retained with hashes as candidate1. Cross-build2 additionally
  reports the unlimited recursion depth from the actual child pipe event rather
  than printing the known requested depth; it compiles with -Wall -Werror.

- memory-native1: actual0passed/1failed/0ignored in35.62s. Init exited16,
  followed by a genuine stopped-process lifecycle refusal, no OOM. Both its
  managed cleanup and diagnostic1 cleanup are empty. Diagnostic1 through the
  actual installed CLI confirms exact4MiB/512KiB/64KiB limits and zero capability
  masks, but reports a successful8MiB malloc. Native ELF disassembly proves the
  actual cause: clang -O2 removed both malloc/free calls for the unobservable
  allocations and folded the limited branch into failure. The explicit mmap
  call remains; no kernel or product failure is inferred. Keep native1 ELF/source
  and diagnostic3 ELF/source/commands/disassembly. Fix probe compilation with
  -fno-builtin-malloc -fno-builtin-free, retain every original effect assertion,
  inspect emitted calls and retry as native2. Box/OCI/client source inputs are
  unchanged; only the external probe ELF must be regenerated and revalidated.
