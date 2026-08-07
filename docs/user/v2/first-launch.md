# First launch

When the Studio opens for the first time on a supported target:

1. The product verifies the offline bundle and the active pack set.
   `Pack Manager` shows the result.
2. If the pack set is missing or unsigned, the product refuses to
   start. Use `Pack Manager → repair from installer payload` and
   point at the offline bundle root.
3. When the pack set is verified, the home screen shows the four
   sample projects under `Open sample`. Each one is small and
   reproducible from the bytes in the offline bundle.

## Packs

* The base app ships with a small capability set so projects can
  open. Model bytes do not pretend to be installed when they are not.
* The Creator offline bundle ships the `media`, `speech`, `director`,
  `vision`, `voice` and `creative` packs. This is the supported
  configuration for the four production lanes.
* A quality pack may be installed separately. It is signed and
  verified by `Pack Manager`; never copy pack bytes outside the
  offline bundle.

## Make Versions

`Make Versions` produces per-format cuts of a single project. The
first launch creates the v2 project root and binds every clip to a
revision. The first version is always `reviewed`; promotion to
`review_light` or `autonomous` requires an explicit per-format user
approval recorded in the decision log.
