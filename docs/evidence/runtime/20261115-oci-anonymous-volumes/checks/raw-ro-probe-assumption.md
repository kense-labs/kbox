# Corrected Guest attack prerequisite

Attempts5/6 kept failed assertions and raw output: BLKROSET(0) succeeded but
BLKROGET still returned1 on exact254:32, so a test assumption that userspace could
clear the virtio hardware RO feature was false. This is not a Host writable bug.
The stronger Guest-device test retains that actual immutable feature as an
assertion and also requires direct raw writes, privileged remount, valid-device
fresh mount and child writes to fail. It does not claim they reach the Host.

[Linux v6.12 ioctl.c](https://raw.githubusercontent.com/torvalds/linux/v6.12/block/ioctl.c)
clears BD_READ_ONLY only. [blkdev.h](https://raw.githubusercontent.com/torvalds/linux/v6.12/include/linux/blkdev.h)
computes bdev_read_only from both the block-device flag and GD_READ_ONLY. This
matches the actual readback, and explains why assuming an EIO backend request
was incorrect. No firmware/kernel/native changes or permissions were added.

Host enforcement remains an independent REQUIRED gate: the real native child
uses production exact-file sandbox + retained descriptors and must produce12
actualEPERM mutations, thread/execEPERM, read-only FD EBADF and a successful RW
control. The actual VM loader/FD maps must also show only read-only native and
retained FDs for the exact RO source. Neither guest RO flags nor an error-only
Guest probe substitutes for this Host gate.


Attempt7 then preserved a second incorrect test expectation: actual raw block
write errno is EPERM (dd copied0bytes), not filesystem-path EROFS. The test now
requires that observed explicit errno description for this exact device profile;
it does not treat arbitrary failure, missing device/command or zero tests as a
pass. Exact device major/minor is checked from real Guest sysfs/proc readback.
