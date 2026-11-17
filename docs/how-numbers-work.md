# How Dimvexa's numbers work

This page explains, with a little mathematics, how Dimvexa stores a number, how it calculates
with it, and how it turns the exact result into a floating-point `f64` without ever guessing.
You need school algebra; every symbol is explained where it first appears. The decision record
behind the current design is [ADR-028](adr/028-powers-of-ten-stay-in-the-exponent.md); the
guarantees are in [precision](precision.md).

- [1. The problem](#1-the-problem)
- [2. How a number is stored](#2-how-a-number-is-stored)
- [3. Reading a decimal such as `6.62607015e-34`](#3-reading-a-decimal-such-as-662607015e-34)
- [4. Calculating: multiply, divide, add](#4-calculating-multiply-divide-add)
- [5. Rounding to `f64`, with a proof](#5-rounding-to-f64-with-a-proof)
- [6. A worked example, step by step](#6-a-worked-example-step-by-step)
- [7. What it costs](#7-what-it-costs)
- [8. What can still overflow](#8-what-can-still-overflow)
- [9. Functions: roots, logarithms, sines](#9-functions-roots-logarithms-sines)
- [10. Where the code is and how it is tested](#10-where-the-code-is-and-how-it-is-tested)
- [11. Glossary and references](#11-glossary-and-references)

## 1. The problem

A unit converter is trusted with numbers that other people build things from. The usual way,
multiplying `f64` values, rounds after every step. The error is tiny, but it is there, and in a
chain of steps it adds up:

$$
E=\frac{h\,c}{\lambda},\qquad h=6.62607015\cdot10^{-34},\ c=299\,792\,458,\ \lambda=5.5\cdot10^{-7}
$$

| | value of $E$ (joule) |
|---|---|
| exact value | 3.61171974027077**9454**…e-19 |
| `h * c / lambda` with `f64` arithmetic | 3.611719740270779e-19 (one step too small) |
| Dimvexa (exact, then rounded once) | 3.6117197402707796e-19 (the nearest `f64`) |

Dimvexa keeps every number **exact** while it calculates, and rounds **once**, at the very end,
to the `f64` that is closest to the exact result. "Exact factors. Loud failures. Zero guessing."
(ADR-002). When it cannot stay exact it says so with an error; it never returns a number it
could not prove.

## 2. How a number is stored

Every number in Dimvexa has the form

$$
x \;=\; \frac{n}{d}\cdot 10^{k}\cdot \pi^{m}
$$

* $n$ and $d$ are integers (128-bit, `i128`), $d>0$, and the fraction is fully reduced
  ($\gcd(n,d)=1$). So $1/3$ and $5/9$ are exact.
* $k$ and $m$ are small integers (32-bit): the exponent of ten and the exponent of $\pi$.

Why three parts?

| Part | Why it exists | Example |
|---|---|---|
| fraction $n/d$ | exact thirds, ninths, any ratio of integers | the foot is $0.3048\,\text{m}=\frac{381}{1250}\,\text{m}$ |
| $10^{k}$ | very large or very small magnitudes without very large digits | the electron-volt is $1602176634\cdot10^{-28}$ J |
| $\pi^{m}$ | angles are exact, never an approximation of $\pi$ | one degree is $\frac{1}{180}\,\pi$ rad |

A number takes 48 bytes (two `i128`, which forces 16-byte alignment, plus two `i32` and padding). The integers
are limited to $|n|,d<2^{127}\approx1.7\cdot10^{38}$, so a fraction can hold **38 decimal
digits**. The exponents $k$ and $m$ are not part of that limit, which is the whole point of
keeping them apart:

> $6.62607015\cdot10^{-34}$ written as a plain fraction is $\dfrac{662607015}{10^{42}}$, and
> $10^{42}$ has 43 digits, more than the 38 an `i128` holds. Written as
> $662607015\cdot10^{-42}$ it needs 9 digits and an exponent.

## 3. Reading a decimal such as `6.62607015e-34`

Take the text apart into the digits $D$ (as one integer), the number $f$ of digits after the
decimal point, and the exponent $e$:

$$
\texttt{6.62607015e-34}:\quad D=662607015,\quad f=8,\quad e=-34
\qquad\Longrightarrow\qquad
x=D\cdot10^{\,e-f}=662607015\cdot10^{-42}
$$

Then there are two ways to store it:

1. **It fits as a fraction.** If $e-f<0$ and $10^{\,f-e}<2^{127}$, store $D/10^{f-e}$ (reduced);
   if $e-f\ge0$ and $D\cdot10^{e-f}<2^{127}$, store the integer. This is how every number was
   stored before, and it still is: nothing that worked changes its representation, its `==`, its
   hash or its printed form.
2. **It does not fit.** Store $n=D,\ d=1,\ k=e-f$. This is the new case (ADR-028). Here
   $10^{42}\ge2^{127}$, so Planck's constant is stored as $n=662607015,\ d=1,\ k=-42$.

The exponent is capped at $|e|\le10\,000$ (an error beyond it), so a hostile text such as
`1e999999999` cannot make anything run long.

## 4. Calculating: multiply, divide, add

### Multiplying

For two numbers $\frac{n_1}{d_1}10^{k_1}\pi^{m_1}$ and $\frac{n_2}{d_2}10^{k_2}\pi^{m_2}$:

$$
g_1=\gcd(n_1,d_2),\quad g_2=\gcd(n_2,d_1),\qquad
\text{product}=\frac{(n_1/g_1)\,(n_2/g_2)}{(d_1/g_2)\,(d_2/g_1)}\cdot10^{\,k_1+k_2}\cdot\pi^{\,m_1+m_2}
$$

Dividing out $g_1,g_2$ **before** multiplying keeps the numbers as small as possible (the result
is already fully reduced, so no second gcd is needed). The exponents simply **add**; they can
never overflow a numerator.

If the integer product still does not fit, Dimvexa moves every factor of ten from $n$ and $d$ into
$k$ (for example $n=2\,000\,000$ becomes $n=2,\ k\mathrel{+}=6$) and multiplies again. This is
exact, and it is the reason a **power of ten can no longer make a product overflow**:

$$
10^{38}\cdot100=1\cdot10^{40}\ \ \text{(exactly)},\qquad
(1.380649\cdot10^{-23})^{2}=1\,906\,191\,661\,201\cdot10^{-58}
$$

The second example is Boltzmann's constant squared. As a fraction it is $\frac{1380649}{10^{29}}$,
and $10^{58}$ would not fit; with the power of ten moved out, the digits need 13 places.

**The order of the factors does not matter.** `ke * e^2 / a0^2`, `ke * e * e / a0^2` and every
other order of those five factors (120 of them) give the same exact value; this is a test
(`a2_every_order_of_the_factors_gives_the_same_value`). The reason is that after this step only
the *digits* decide whether a product fits, and digits are the same whatever the order. (The
digits can still be too many: section 8.)

Division is multiplication by the reciprocal: $\left(\frac{n}{d}10^{k}\pi^{m}\right)^{-1}=\frac{d}{n}10^{-k}\pi^{-m}$.

### Adding and comparing

Adding needs a common form:

$$
a\cdot10^{k_1}+b\cdot10^{k_2}\quad\text{needs both on one exponent, say } k_1\le k_2:\qquad
\bigl(a+b\cdot10^{\,k_2-k_1}\bigr)\cdot10^{k_1}
$$

which only works when $b\cdot10^{k_2-k_1}$ fits the integer. The core refuses sums and
comparisons of unlike exponents with `unrepresentable` (ADR-019, KL-24): for $\pi$ it is a real
impossibility ($1+\pi$ has no exact form, and Dimvexa never approximates). `dimvexa-calc` first
folds the power of ten into the fraction, which succeeds whenever the *result* fits:

$$
1\ \text{J}+1\ \text{eV}=\Bigl(1+1602176634\cdot10^{-28}\Bigr)\ \text{J}\quad\text{(exact)}
$$

## 5. Rounding to `f64`, with a proof

### 5.1 What an `f64` is

An `f64` is a number $(-1)^{s}\,(1+\tfrac{F}{2^{52}})\,2^{E}$ with 53 significant bits. Only
finitely many numbers have that form, so an exact $x$ usually falls **between two neighbours**.
*Correct rounding* means returning the nearer one (on an exact tie, the one with an even last
bit). Write it $\operatorname{rn}(x)$. Two facts matter:

* $\operatorname{rn}$ is **monotone**: if $a\le b$ then $\operatorname{rn}(a)\le\operatorname{rn}(b)$.
* Chaining `f64` operations does *not* give $\operatorname{rn}$ of the exact result (section 1).

### 5.2 The goal and the obstacle

We want $\operatorname{rn}(x)$ for $x=\frac{n}{d}10^{k}\pi^{m}$, **without** ever holding $x$
exactly: $10^{k}$ may have hundreds of digits and $\pi$ has infinitely many.

### 5.3 The idea: sandwich the value, then check

Compute two numbers $L$ and $U$ that are **guaranteed** to satisfy

$$
L\;\le\;x\;\le\;U
$$

and are very close together. Because $\operatorname{rn}$ is monotone,

$$
\operatorname{rn}(L)\;\le\;\operatorname{rn}(x)\;\le\;\operatorname{rn}(U)
$$

so **if $\operatorname{rn}(L)=\operatorname{rn}(U)$, that common value is $\operatorname{rn}(x)$** — proven,
not estimated. If they differ, $x$ is within the gap $U-L$ of a rounding boundary; Dimvexa then
reports an error (or falls back to an exact method), and never returns a guess.

### 5.4 How $L$ and $U$ are computed: 128-bit mantissas and directed rounding

A positive number is held as $m\cdot2^{e}$ with a 128-bit integer $m$ in $[2^{127},2^{128})$
(type `Fx`). Multiplying two such numbers gives a 256-bit product; keeping the top 128 bits and
**rounding down** gives a result $\le$ the true product, **rounding up** a result $\ge$ it. Each
operation loses less than one unit in the 128th bit:

$$
\left|\frac{\text{rounded}-\text{true}}{\text{true}}\right|<2^{-127}\quad\text{per operation}
$$

Splitting the exponent into its positive and negative parts, $k^{+}=\max(k,0)$ and
$k^{-}=\max(-k,0)$ (and the same for $m$), the value is a quotient of two products:

$$
x=\frac{N}{D},\qquad N=n\cdot10^{k^{+}}\pi^{m^{+}},\qquad D=d\cdot10^{k^{-}}\pi^{m^{-}}
$$

$$
L=\frac{\lfloor N\rfloor}{\lceil D\rceil}\ \text{(rounded down)},\qquad
U=\frac{\lceil N\rceil}{\lfloor D\rfloor}\ \text{(rounded up)}
$$

where $\lfloor\cdot\rfloor,\lceil\cdot\rceil$ are the 128-bit lower and upper bounds. A few dozen
operations at $<2^{-127}$ each keep $U/L-1$ below about $2^{-120}$.

**How often does it fail?** The midpoints between neighbouring `f64` are about $2^{-52}$ apart
relatively, so the chance that a random $x$ lies within $2^{-120}$ of one is about
$2^{-120}/2^{-52}=2^{-68}$, roughly one in $10^{20}$. In 194 000 random decimals compared with
the standard library it never happened (`exact_f64.rs`). When it does, the error is explicit.

### 5.5 Where the time went, and the three speed-ups

Before ADR-028 the bounds were built the slow way. Now:

**(a) A table for $10^{k}$.** $10^{k}$ was rebuilt for every call by repeated squaring,

$$
10^{k}=\prod_{i}\bigl(10^{2^{i}}\bigr)^{b_i}\quad\text{where } k=\sum_i b_i2^{i},\ b_i\in\{0,1\},
$$

about $\log_2k$ wide multiplications for each of the two bounds. Now the pair of bounds
$\bigl(\lfloor10^{k}\rfloor,\lceil10^{k}\rceil\bigr)$ for $k=0\ldots350$ (the range of an `f64`,
$10^{\pm308}$, plus room for the digits) is computed **once** and looked up. For $k\le55$ the
bound is exact ($10^{55}=2^{55}\cdot5^{55}$ and $5^{55}<2^{128}$). The table is a fixed array of
351 entries of 80 bytes, **28,080 bytes**, built on first use with no heap allocation.

**(b) Dividing two 128-bit mantissas in two steps.** The quotient of two normalised mantissas
$a<b$ with $2^{127}\le b<2^{128}$ is a 128-bit number

$$
q=\left\lfloor\frac{a\cdot2^{128}}{b}\right\rfloor
$$

which was produced one bit at a time, 128 shift-compare-subtract rounds per bound. Now it is
produced as **two 64-bit digits** in base $B=2^{64}$ (Knuth's *Algorithm D*). Write
$b=b_1B+b_0$, with $b_1\ge B/2$ because $b\ge2^{127}$. For the current remainder $r$ the digit is

$$
\hat q=\left\lfloor\frac{(r\cdot B)\ \text{top 128 bits}}{b_1}\right\rfloor,\qquad
q\ \le\ \hat q\ \le\ q+2
$$

(Knuth, *TAOCP* vol. 2, §4.3.1, Theorem B: the estimate from the top limb is never too small
and at most 2 too large when the divisor is normalised.) The code lowers $\hat q$ until
$\hat q\cdot b\le r\cdot B$ holds exactly, subtracts, and repeats once more. The result is
**bit-identical** to the old loop (tested on two million random and edge-case inputs against the
loop, which stays in the tests).

**(c) The fast route first.** `Ratio::to_f64` used to fold the power of ten into the fraction
first, which costs several 128-bit gcds. It now tries the bounds first (about 50 ns) and folds
only if they cannot decide. Both routes return the same bits (tested on over 100 000 random values).

```
to_f64(x = n/d · 10^k · pi^m)
 ├─ 1. bounds L and U, from the table            ≈ 50 ns   proven, or...
 ├─ 2. fold 10^k into the fraction, round        ≈ 200 ns  exact, if it fits i128
 └─ 3. error `overflow`                                    never a guess
```

### 5.6 Relation to Clinger and Eisel–Lemire

Parsing a decimal into the nearest `f64` is a classic problem, and the same ingredients appear
in the best-known solutions:

* **Clinger (1990)** noticed that if the digits are small ($D<2^{53}$, so exactly an `f64`) and
  $|q|\le22$ (so $10^{|q|}$ is exactly an `f64`), then $D\cdot10^{q}$ or $D/10^{|q|}$ is a
  *single* correctly rounded `f64` operation. Fast, but only for small numbers.
* **Eisel–Lemire (Lemire 2021)** handles every 64-bit $D$ and every $q\in[-342,308]$ with a
  **table of 128-bit powers of ten** (their $10^{q}$ or $5^{q}$ entries, truncated) and **one
  wide multiplication**. It then checks whether the low bits leave the rounding *ambiguous*;
  if they do, it falls back to a slow exact method. "Table, one multiplication, a proof that
  rounding is decided, a slow fallback" is the pattern.

Dimvexa adopts the pattern and changes two things because its inputs are richer than a decimal
string: the value is a **fraction** $n/d$ and may carry $\pi^{m}$, so it needs a division and
the power of $\pi$ (`PI_LO`, a 128-bit lower bound of $\pi$ checked against Machin's formula),
and instead of one truncated product it keeps a **lower and an upper bound**, so the proof of
section 5.3 is explicit and the fallback is either exact folding or an error.

## 6. A worked example, step by step

$E=h\,c/\lambda$ with $h=6.62607015\cdot10^{-34}$, $c=299\,792\,458$, $\lambda=5.5\cdot10^{-7}$.
These are the values the library really holds (checked by running it):

| Step | Value | Stored as $(n,\ d,\ k)$ |
|---|---|---|
| parse $h$ | $662607015\cdot10^{-42}$ (does not fit a fraction) | $(662607015,\ 1,\ -42)$ |
| parse $c$ | $299792458$ | $(299792458,\ 1,\ 0)$ |
| parse $\lambda$ | $5.5\cdot10^{-7}=\frac{11}{20\,000\,000}$ (fits) | $(11,\ 20000000,\ 0)$ |
| $h\cdot c$ | $198\,644\,585\,714\,892\,870\cdot10^{-42}$ | $(198644585714892870,\ 1,\ -42)$ |
| $\div\lambda$ | multiply by $\frac{20\,000\,000}{11}$ | $(3972891714297857400000000,\ 11,\ -42)$ |

The last numerator has 25 digits and the denominator is 11; the power $10^{-42}$ travelled
along as an exponent and never touched them. To round it, split $x=N/D$ with $N=3\,972\,891\,714\,297\,857\,400\,000\,000$
and $D=11\cdot10^{42}$ (the negative exponent goes to the denominator side):

1. look up the bounds of $10^{42}$ in the table (one lookup, not six squarings);
2. $\lfloor D\rfloor=11\cdot\lfloor10^{42}\rfloor$ and $\lceil D\rceil=11\cdot\lceil10^{42}\rceil$ as 128-bit numbers;
3. $L=\lfloor N\rfloor/\lceil D\rceil$ and $U=\lceil N\rceil/\lfloor D\rfloor$, each by a two-digit division;
4. round $L$ and $U$ to 53 bits: both give `3.6117197402707796e-19`, so that is $\operatorname{rn}(x)$.

The exact value is $3.611\,719\,740\,270\,779\,454\,5\ldots\cdot10^{-19}$; the two neighbouring `f64`
are $3.6117197402707791\ldots$ and $3.6117197402707796\ldots$ (times $10^{-19}$), and the second
is closer. Plain `f64` arithmetic returns the first.

## 7. What it costs

Per call, release build, best of five runs, in the cloud container this was developed in
(Linux x86-64, shared machine, so read small differences as noise). The `to_f64` rows come
from a micro benchmark of `Ratio::to_f64`; the guarded row comes from the repository's
`cargo xtask bench-impact`:

| `Ratio::to_f64` of | before | after |
|---|---|---|
| plain fraction ($15/1000$) | 31 ns | 31 ns |
| a power of ten that folds ($1602176634\cdot10^{-28}$) | 209 ns | 46 ns |
| a power of ten that needs the bounds ($662607015\cdot10^{-42}$) | 432 ns | 54 ns |
| with a denominator ($\tfrac67\cdot10^{-50}$) | 431 ns | 44 ns |
| a power of $\pi$ ($\tfrac1{180}\pi$) | 379 ns | 51 ns |

The guarded `deg -> rad` benchmark row (`apply_exact_f64`) went from 448 ns to 103 ns. The fast
`f64` path (`apply`, `apply_slice`), used for bulk data, does not call any of this and did not
change. Memory: a number is still 48 bytes; the table is 28,080 bytes once per process.

## 8. What can still overflow

A power of ten no longer can. What remains is the number of **digits** in $n$ and $d$, at most 38.
These still fail with `overflow`, loudly:

| Case | Why |
|---|---|
| `1 ly^3` to $\text{m}^3$ | $9\,460\,730\,472\,580\,800^3$ has 48 digits |
| `1 psi^3` | the factor of `psi` is $\frac{8896443230521}{1290320000}$ (13 and 10 digits); cubed it needs 39 |
| $2\pi^5k^4/(15\,h^3c^2)$ from the constants | $662607015^3\cdot299792458^2$ has 43 digits |

This is an accepted limit of the exact core (owner decision, 2026-11-16; KL-31 in
[known limitations](known-limitations.md), pinned by a test), not a bug to be fixed. Wider
integers would cost every calculation (a larger and slower number, a new public type) for the
rare ones that need them, and cancelling common factors across a product does not rescue these
cases: $k^4/(h^3c^2)$ still has 40 digits in its reduced denominator. What helps today is
keeping the *powers of ten* out of the digits, which is free: an SI prefix on the input
(`0.000662607015 qJ*s`) or on the result unit (`nN`, `pm^2`). A separate, labelled approximate
tier remains possible (ADR-027) and would never mix silently with exact values.

## 9. Functions: roots, logarithms, sines

Everything so far is exact: a result is a fraction (times powers of ten and $\pi$). But
$\sqrt2$, $\ln3$ and $\sin45^\circ$ are not fractions, and engineering formulas need them (the
orifice flow $Q=C_dA\sqrt{2\,\Delta p/\rho}$, the log mean temperature difference
$\Delta T_{lm}=(\Delta T_1-\Delta T_2)/\ln(\Delta T_1/\Delta T_2)$, the force along a slope
$mg\sin\theta$). `dimvexa-calc` has them (ADR-029) under one rule:

> **A result is exact whenever the mathematics makes it exact. Otherwise it is an approximate
> value, labelled `≈`, and everything computed from it is labelled too.**

```
sqrt(9 m^2)        = 3 m                       exact
sin(30 deg)        = 1/2                       exact
8^(2/3)            = 4                         exact
sqrt(2 m^2)        ≈ 1.4142135623730951 m      approximate, labelled
ln(3)              ≈ 1.0986122886681098        approximate, labelled
```

### 9.1 When a root is exact

For $x=\dfrac{n}{d}\cdot10^{k}\pi^{m}$ and a root of index $q$:

$$
\sqrt[q]{x}\;=\;\frac{\sqrt[q]{n}}{\sqrt[q]{d}}\cdot10^{k/q}\cdot\pi^{m/q}
$$

is again of the stored form exactly when $n$ and $d$ are perfect $q$-th powers, and $q$ divides
$m$ and the exponents of ten. A fraction has a rational $q$-th root only if $n$ and $d$ do (in
lowest terms), so no exact case is missed. A power $x^{p/q}$ is the $q$-th root raised to the
integer power $p$. The integer root is found by bisection with `checked_pow`.

The power of ten needs one more step. Since $10^{k}=2^{k}\cdot5^{k}$, a value such as
$1000\cdot10^{-101}$ is really $10^{-98}$, with the exact root $10^{-49}$, although its stored
exponent $-101$ is odd; and $\tfrac25\cdot10^{1}=4$ has the root 2. So the 2s and 5s are taken
out of $n$ and $d$ and counted together with $k$:

$$
\alpha=v_2(n)-v_2(d)+k,\qquad \beta=v_5(n)-v_5(d)+k,\qquad
\text{exact}\iff q\mid\alpha,\ q\mid\beta,\ q\mid m,\ \text{and the rest of }n,d\text{ are }q\text{-th powers}
$$

where $v_p$ counts the factors $p$. A randomised test checks that the root of every perfect
square and cube is found (`the_root_of_a_perfect_power_is_always_exact`).

### 9.2 Units under a root

Dimensions are vectors of integer exponents (`[kg, m, s, A, K, mol, cd]`), so a root is allowed
only when every exponent stays an integer:

$$
\sqrt{\text{m}^{2}\,\text{s}^{-2}}=\text{m}\,\text{s}^{-1}\ \checkmark\qquad
\sqrt{\text{m}}=\text{m}^{1/2}\ \times\ (\text{refused, KL-32})
$$

The value is first converted to SI base units (`16 km^2` is $16\cdot10^{6}\ \text{m}^2$), the
exponents are scaled by $p/q$, and the result is in SI base units (`4000 m`). A dimensionless
ratio has no such limit, which is how Manning's $R^{2/3}$ is written: $(R/1\,\text{m})^{2/3}$.

### 9.3 Logarithm, exponential, sine, cosine

By the Hermite–Lindemann theorem, $e^{x}$ is transcendental for every non-zero algebraic $x$, and
so $\ln x$ is for every algebraic $x\ne0,1$. Hence the *only* exact cases are

$$
\ln 1=0,\qquad e^{0}=1
$$

Sine and cosine of an angle that is a rational multiple of $\pi$ (every `deg`, `grad`, `arcmin`
is) are rational only at multiples of $30^\circ$, and then only $0,\pm\tfrac12,\pm1$ (Niven's
theorem, 1956), so the table is complete:

| $\theta$ | $0^\circ$ | $30^\circ$ | $90^\circ$ | $150^\circ$ | $180^\circ$ | $210^\circ$ | $270^\circ$ | $330^\circ$ |
|---|---|---|---|---|---|---|---|---|
| $\sin\theta$ | 0 | $\tfrac12$ | 1 | $\tfrac12$ | 0 | $-\tfrac12$ | $-1$ | $-\tfrac12$ |

with $\cos\theta=\sin(\theta+90^\circ)$. At $60^\circ$ or $45^\circ$ the value is $\tfrac{\sqrt3}2$ or
$\tfrac{\sqrt2}2$, which is irrational, so it is approximate. The test is exact arithmetic: an
angle $\theta=\tfrac{n}{d}10^{k}\pi$ is $\theta/\text{deg}=\tfrac{n}{d}10^{k}\cdot180$ degrees; it is a
multiple of $30^\circ$ when that number divided by 30 is an integer. A bare number is refused:
`sin(0.5)` could be radians or degrees, and the library does not guess.

### 9.4 What "approximate" means here

An approximate value is an `f64` with its unit. It is made only when the exact result does not
exist. From then on:

* **It spreads.** Any operation with an approximate operand is approximate; it never becomes
  exact again. `sqrt(2)^2` is `≈ 2.0000000000000004`, not `2`.
* **Exact failures stay failures.** If the *exact* arithmetic overflows (the digit limit of
  section 8), the answer is `overflow`, never an `f64` that hides it.
* **A function that does not exist is an error that says so.** `tan`, `asin`, `log10`, `atan2`,
  `hypot` and the others that are not implemented fail with `'tan' is not implemented` (code
  `refused`, as for the other unsupported requests) and, where there is a way, how to write
  them from the functions that are there: `sin(x) / cos(x)`, `ln(x) / ln(10)`, `x^(1/3)`.
* **Accuracy is stated, not promised.** Arithmetic and `sqrt` are IEEE operations (at most half
  a unit in the last place per step), a unit conversion uses the correctly rounded exact factor,
  and `ln`, `exp`, `sin`, `cos` and general powers come from the platform's maths library, which
  promises no correct rounding (about 1 ulp is typical). The errors of a chain add up and are
  not tracked (KL-33). If you need a bound, use the exact tier or compute one separately.

## 10. Where the code is and how it is tested

| What | Where |
|---|---|
| the number type, parsing, multiplication, `to_f64` | `crates/dimvexa-core/src/ratio.rs` |
| bounds, the table, the division, the proof | `crates/dimvexa-core/src/approx.rs` |
| 256-bit helper for the wide product | `crates/dimvexa-core/src/wide.rs` |
| sums, comparisons, temperatures | `crates/dimvexa-calc/src/lib.rs` |
| `sqrt`, powers `p/q`, `ln`, `exp`, `sin`, `cos`, the approximate value | `crates/dimvexa-calc/src/math.rs` |

| Claim | Test |
|---|---|
| the new division equals the old bit loop | `word_division_matches_bit_division` (2 million inputs, `approx.rs`) |
| bounds and folding agree bit for bit | `bracketing_agrees_with_folding_where_both_decide` (`approx.rs`) |
| rounds like the standard library (independent oracle) | `decimals_with_a_far_exponent_round_like_the_standard_library` (`exact_f64.rs`, 194 000 random decimals) |
| every factor order gives one value | `a2_every_order_of_the_factors_gives_the_same_value` (`engineering/number_range.rs`) |
| hostile exponents stay bounded | `a_decimal_exponent_is_kept_up_to_the_cap_and_refused_beyond_it` (`hostile_inputs.rs`) |
| the new form of `6.62607015e-34` | `a_power_of_ten_that_does_not_fit_stays_in_the_exponent` (`ratio.rs`) |
| the exact roots are complete | `the_root_of_a_perfect_power_is_always_exact`, `a_perfect_square_hidden_in_the_stored_power_of_ten_is_still_exact` (`crates/dimvexa-calc/tests/math.rs`) |
| real engineering equations | `crates/dimvexa-testkit/tests/engineering/` (one file per field) |

## 11. Glossary and references

| Term | Meaning |
|---|---|
| exact number | $\frac{n}{d}10^k\pi^m$ with integer $n,d,k,m$ |
| mantissa | the significant digits, here a 128-bit integer $m$ in $m\cdot2^{e}$ |
| ulp | unit in the last place: the gap between two neighbouring `f64` |
| directed rounding | rounding always down (for a lower bound) or always up (for an upper bound) |
| correct rounding | returning the nearest `f64` to the exact value, ties to even |
| limb | one 64-bit "digit" of a larger integer |
| fold | multiply $10^{k}$ into the fraction: $\frac{n}{d}10^k\to\frac{n\,10^k}{d}$ |

References (standard works; the algorithms are described there, the code here is Dimvexa's own):

* W. D. Clinger, *How to read floating point numbers accurately*, PLDI 1990.
* D. Lemire, *Number parsing at a gigabyte per second*, Software: Practice and Experience 51(8),
  2021 (the Eisel–Lemire algorithm, with M. Eisel).
* D. E. Knuth, *The Art of Computer Programming*, vol. 2, §4.3.1, Algorithm D.
* D. Goldberg, *What every computer scientist should know about floating-point arithmetic*,
  ACM Computing Surveys 23(1), 1991.
* I. Niven, *Irrational Numbers*, Mathematical Association of America, 1956 (rational values of
  the sine at rational multiples of $\pi$); the Hermite–Lindemann theorem (transcendence of
  $e^{x}$ for non-zero algebraic $x$).
