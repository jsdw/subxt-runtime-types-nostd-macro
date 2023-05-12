# subxt-runtime-call-macro (no_std)

A work experiment to use `subxt-codegen` to provide a macro which generates the runtime types given some metadata in a `no_std` compatible way.

Notes:
- This is quite hacky; `subxt-codegen` (and `subxt-metadata`) are both tuned towards generating code that works well for Subxt. To work around this, we have to mimic some `subxt` deps that the codegen depends on.
- In this example, a couple of external crates (here, `bitvec` and `parity-scale-codec`) must be depended on by any crate using the macro. They need specific feature flags enabled to play nicely. We should probably create a non-macro crate to export the macro through (as we do in Subxt) so that we can pull in what we need there.
- The APIs that we use from the codegen crate aren't super clean and are typically things that I've considered part of an internal interface.
- This macro crate relies on a branch of `subxt-codegen` at the moment which adds a feature flag to toggle between generating `std` and `no_std` compatible output (I can't see a way of supporting both without the flag offhand alas).

All that said, this example seems to show that it can work. Go into `no_std_test` to run a test (see the readme there) which uses the macro in a `no_std` environment successfully.

In the long run, I think a more robust approach might be to write custom code to generate what we need here, but perhaps this can serve as a stopgap until such a time as there is bandwidth to work on this. Or, extract the type generation logic into a separate crate, improve the APIs with both use cases in mind, and make use of it in both places.