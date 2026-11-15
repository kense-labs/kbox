RawRO9 passed the installed native probe but its first loader-time descriptor
snapshot contained only the retained RO pin, before libkrun opened its own disk
FD. The additional receipt assertion requiring both actual FDs therefore failed.
That initial snapshot is retained and is not a complete native-backend FD proof.
The diagnostic runner now keeps that snapshot and separately waits for both
retained/native descriptors, checks all RO descriptors are r and RW descriptors
are u, and preserves the later native snapshot too. New finalRawRO10 required.
No product source/assets or test assertions changed for this sampler correction.
