---
type: knowledge
metadata:
  title: "xiuxian-core"
---

# xiuxian-core

Microkernel core for xiuxian-artisan-workshop.

## Components

- `xiuxian_core.kernel`: Microkernel abstraction layer (Kernel, Lifecycle, Components)

## Dependencies

- `xiuxian-foundation`: Foundation layer (logging, config, thin helpers)
- `wendao-core-lib`: Arrow Flight transport/client surface

## Usage

```python
from xiuxian_core.kernel import get_kernel

kernel = get_kernel()
await kernel.initialize()
```
