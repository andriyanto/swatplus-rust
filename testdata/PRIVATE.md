# The private scenarios

Two of the four validation projects, `Batanghari` (1,858 HRUs) and `Pessel` (3,966 HRUs), both in
West Sumatra, Indonesia, are private data and are not in this repository. They were provided by the
Water Resources and Construction Development Agency (Dinas Sumber Daya Air dan Bina Konstruksi) of
the Provincial Government of West Sumatra and may not be redistributed.

They are kept in the private repository `andriyanto/swatplus-testdata`. The `Validate against
SWAT+` workflow checks them out into `testdata/` with a read-only token (the `TESTDATA_TOKEN`
secret); without it, as in a fork's pull request, only SWAT+'s own scenarios (`Ames_sub1`,
`Osu_1hru`) and the variants built from them run. The editor and reviewers of a journal can be
given confidential access, with the agency's agreement.

The summaries of their comparisons, which hold counts and differences but no values, are public:
`docs/validation-results/` and `docs/benchmark-results/`.
