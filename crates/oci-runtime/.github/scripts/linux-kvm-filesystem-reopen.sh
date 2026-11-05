#!/usr/bin/env bash
set -Eeuo pipefail

source .github/scripts/lib/linux-kvm-mutation-reopen.sh

linux_kvm_mutation_reopen \
  filesystem \
  linux-kvm-filesystem-reopen \
  krun.oci.linux-kvm-filesystem-reopen-matrix.v1 \
  linux-kvm-filesystem-reopen-9-stage-v1 \
  "${KRUN_OCI_LINUX_KVM_FILESYSTEM_REOPEN_REPORT:-}" \
  kvm-filesystem-reopen- \
  krun-oci-kvm-filesystem-reopen
