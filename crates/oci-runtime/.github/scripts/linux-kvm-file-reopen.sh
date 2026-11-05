#!/usr/bin/env bash
set -Eeuo pipefail

source .github/scripts/lib/linux-kvm-mutation-reopen.sh

linux_kvm_mutation_reopen \
  file \
  linux-kvm-file-reopen \
  krun.oci.linux-kvm-file-reopen-matrix.v1 \
  linux-kvm-file-reopen-9-stage-v1 \
  "${KRUN_OCI_LINUX_KVM_FILE_REOPEN_REPORT:-}" \
  kvm-file-reopen- \
  krun-oci-kvm-file-reopen
