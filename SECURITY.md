# Security policy

Dimvexa is a unit-conversion library. Its job is to give a correct number or a loud error,
never a plausible wrong number. We therefore treat anything that can make it fail silently
as a security-relevant bug, and wrong results are taken very seriously.

## Supported versions

Dimvexa is pre-1.0. Only the **latest release** receives fixes. Before the first release,
only the `main` branch is supported. Older `0.x` versions are not patched; upgrade to the
latest one.

## Reporting a vulnerability

Report **privately**. Do not open a public issue, pull request or discussion for a
vulnerability.

1. Go to <https://github.com/welworx/dimvexa/security/advisories/new>, or open the
   repository's **Security** tab and choose **Report a vulnerability**.
2. Describe the problem (see "What to include" below) and submit.

The report is visible only to the maintainer until a fix is ready. If the form is not
available to you, open a normal issue that says only "I need a private channel for a
security report" and contains no details; the maintainer will arrange one.

## What counts as a security issue

Please report privately:

- A crafted unit string, number or data file that causes a **hang**, super-linear running
  time, **unbounded memory use**, a **panic** or an abort. The library promises to return an
  error for every input.
- An **undetected overflow** or other arithmetic fault that returns a wrong result without an
  error (the exact path must fail with `overflow`, never wrap or fall back to floats).
- A **silent wrong result** caused by the code, for example a kind or dimension check that
  lets an invalid conversion through, an ambiguous symbol that is resolved without being
  asked, or a fast path that exceeds its documented error bound (`docs/precision.md`).
- Any use of `unsafe` or an undeclared network or file access (neither should exist).
- A **supply-chain or release problem**: a tampered or unexpected published artifact, a
  dependency with a known advisory that affects users, a workflow that could be abused to run
  untrusted code with write access, or leaked credentials.

## What is an ordinary bug

Please use a [normal issue](https://github.com/welworx/dimvexa/issues/new/choose):

- A **wrong conversion factor in the data table**. This is a correctness bug and an important
  one, but it is public by nature and fixed through a reviewed data change. Cite the primary
  source (BIPM, NIST, a statute) in the issue so it can be checked and fixed quickly.
- A missing unit (use the *Unit request* form), a confusing message or a documentation error.

If you are unsure which category applies, report privately; we will move it if needed.

## What to include

- The version or commit, the Rust version and the operating system.
- A minimal reproducer: for the parser, the exact input string or file; for arithmetic, the
  exact values and units.
- What you expected and what happened (output, error code, memory or time used).
- Your assessment of the impact, if you have one.

## What to expect

This is a volunteer-run project, so these are intentions, not guarantees:

- An acknowledgement within about one week.
- An assessment and a plan or a fix timeline as soon as the problem is reproduced.
- A fix released first, then a public GitHub security advisory (with a CVE where
  appropriate) that describes the problem and the fixed version. We ask for coordinated
  disclosure and will agree a date with you; a reasonable default is up to 90 days.

## Credit

Reporters are credited in the advisory and in the changelog unless they prefer to stay
anonymous. There is no bug bounty.
