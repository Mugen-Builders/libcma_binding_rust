#pragma once

// ABI layer (libcmt)
#include "./third_party/machine-guest-tools/sys-utils/libcmt/include/libcmt/abi.h"
#include "./third_party/machine-guest-tools/sys-utils/libcmt/include/libcmt/buf.h"
#include "./third_party/machine-guest-tools/sys-utils/libcmt/include/libcmt/io.h"
#include "./third_party/machine-guest-tools/sys-utils/libcmt/include/libcmt/keccak.h"
#include "./third_party/machine-guest-tools/sys-utils/libcmt/include/libcmt/merkle.h"
#include "./third_party/machine-guest-tools/sys-utils/libcmt/include/libcmt/rollup.h"
#include "./third_party/machine-guest-tools/sys-utils/libcmt/include/libcmt/util.h"

// CMA layer
#include "./third_party/machine-asset-tools/include/libcma/parser.h"
#include "./third_party/machine-asset-tools/include/libcma/types.h"
#include "./third_party/machine-asset-tools/include/libcma/ledger.h"