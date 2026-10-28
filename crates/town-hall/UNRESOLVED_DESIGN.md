# Town Hall — unresolved design issues and research directions

Reviewed against the working tree on 2026-10-18, including the existing edits to
configuration identity and immutable need targets. This is a design review and
proposed research agenda. Recommendations below are proposals, not implemented
behavior or newly settled rules.

Town Hall's strongest possibility is an economy in which people discover both
**what is useful to make** and **whom it is useful to understand**. Exact physical
configurations give invention something substantive to change. Private neural
policies, readers of other policies, and generated social connections give
coordination an unusual mechanism. The interesting result would be that these
two systems improve one another: production creates reasons to understand other
people, and social understanding helps people discover viable production.

The central risk is that the two systems remain impressive but disconnected.
Random policies moving money do not yet demonstrate adaptation; a registry of
conserved configurations does not yet demonstrate an economy. The next work
should establish a small, explainable connection between physical possibility,
individual benefit, exchange, and adaptation.

## Foundations worth protecting

- **Physical identity and economic usefulness are separate.** A good can exist
  as a design before anyone wants it, knows how to manufacture it, or owns a
  unit. Needs refer to ideal configurations independently of `GoodId`. This
  allows unsuccessful inventions, imperfect substitutes, and changing product
  success without changing the underlying identities.
- **Structure survives accounting.** Goods are exact expanded resource
  sequences, stored as runs. Adjacent identical runs merge; separated runs and
  order remain significant. `50 × [(A, 1)]` and `1 × [(A, 50)]` have equal
  primitive material but different physical holdings. Combining, splitting,
  and rearranging require recipes and feasible processes. This distinction is
  the foundation for manufacturing to add value.
- **Recipes and processes answer different questions.** Recipes conserve
  embodied primitive material; processes specify practical requirements.
  Several production techniques can compete to make the same product without
  inventing hidden quality differences within a `GoodId`. Consumer similarity
  does not automatically authorize a substitute input or piece of equipment.
- **People have persistent preferences and private interpretations.** Need
  targets are fixed for an entity's lifetime. Changing intensity, inventories,
  prices, knowledge, and opportunities can change behavior around those fixed
  targets. Each person has private networks and an observer-specific reader;
  shared technological knowledge need not imply shared judgment.
- **The neural architecture takes representation seriously.** The Monomial
  reader removes hidden-neuron permutations and positive rescalings, within
  numerical tolerance. Stable social slots and staged connection installation
  avoid some arbitrary effects of storage layout and evaluation order. These
  are valuable foundations for controlled experiments.

Retain the existing contracts: 256 primitive `u8` resource identities with
coordinates derived from a property lattice; universal indivisible `u32`
quantities; non-empty goods and need targets with positive components; immutable
configuration access; permanent good identities; unordered recipe input
multisets; exact equipment requirements; and the private-network, reader,
fixed-slot, generated-connection architecture. Adjacent good-run overflow still
panics in every build; search must respect that boundary. The initial design
also retains global technological knowledge and independent satisfaction of
all matching needs by one consumed good. Changing either requires an explicit
later experiment. Taste drift is outside this design.

## The implemented boundary

These distinctions matter because some earlier descriptions have fallen behind
the source. The source links identify the basis for this review; proposed
experiments have not been run.

| Area | What exists now | Consequence for the design |
| --- | --- | --- |
| [Goods and recipes](src/market/mod.rs) | Validated, canonical good registration; 256 raw unit designs; conserved recipes for one output unit; process registration. | Registration creates knowledge, not stock. Registered processes are not yet proven feasible or executable. |
| [Process requirements](src/market/recipes.rs) | Workers, duration, exact equipment, consumables, and inefficiency inputs. Zero workers and zero duration are accepted. | These are descriptions, not a technological constraint system or job scheduler. |
| [World generation](src/world/generate.rs) | Seeded deposits independent of the population RNG stream; accessible starting material at the first spawn tile. | Extraction, regeneration, movement, and economy-wide reachability remain absent. Deposit gear is an abstract number, not equipment. |
| [Needs and survival](src/entity/mod.rs) | Each update adds recovery linearly; any need at or above `1.0` makes the entity dead. | Recovery is not bounded. Survival already affects the simulation and needs an explicit policy now. |
| [Tick and social machinery](src/village.rs) | Recovery and pruning precede social reading; new acquaintances receive generated heads; actions use a money snapshot. | The default recovery rate is `0.05` from zero: people reach the death threshold on tick 20 with no satisfaction path. |
| [Neural readers](src/nfn/monomial.rs) | Canonicalized pooled features, private affine parameters, and tests of selected symmetry transformations. | These tests establish useful invariances, not behavioral understanding. There is no objective-driven training or evolutionary selection loop. |
| [Clients](../../apps/town-hall-cli/src/main.rs) and [dashboard](../town-hall-gui/src/lib.rs) | Native clients run `Village`; the browser runs a separate scripted preview. | Browser activity is not evidence about the native economy. Current displays lack decision and accounting explanations. |

The [workspace README](../../README.md) says there are no death rules; the live
code and pruning tests say otherwise. The comment above `baseline_person_needs`
also says roughly 1,000 updates, inconsistent with `0.05`. Resolve this
source/documentation disagreement before interpreting longer runs. The issues
below describe current behavior rather than treating those statements as
implemented guarantees.

## Decisions that determine whether the model works

### D01 — Choose the first claim to demonstrate, and make it survivable

**Critique.** The earlier list made almost every subsystem equally essential.
That encourages building breadth before discovering whether the central
mechanisms work. The current default population also disappears before any
future production or learning experiment could reasonably be interpreted.

**Direction.** Start with a small native scenario in which a person repeatedly
extracts material, makes a structured good, consumes it, and gains more useful
satisfaction per scarce input than through an available raw substitute. Then
introduce a second person with a complementary opportunity and demonstrate a
mutually useful exchange. A diagnostic controller may establish feasibility;
the private neural controllers must later achieve the same task through the
same action and observation boundaries.

Define a tick's duration before calibrating recovery or labor. A candidate
bounded recovery law is `h' = 1 - (1 - h) * exp(-r * dt)`, with `r` expressed per
unit time. It composes consistently across time subdivisions when there are no
intervening events. Starting below one, it approaches one asymptotically, so
mortality needs its own rule instead of depending on numerical rounding.

Choose explicitly between a nonlethal experimental preset and a separately
specified survival rule with a reachable consumption path. Do not silently
retain unavoidable extinction, grant unexplained satisfaction, or infer
successful selection from everyone dying together.

**Evidence needed.** Several complete demand/recovery cycles, a material
ledger, and a trace of why production beats the available alternative. No
market, learning, or survival claim should depend on an empty population.

### D02 — Make the resource geometry economically informative

**Critique.** `DIMENSIONS = 8` divides eight ID bits into eight binary axes.
The resources are the 256 vertices of a cube, and squared resource distance is
Hamming distance divided by eight. That is a particular coarse geometry, not a
densely sampled continuous material space. With equal axis weights, the current
baseline ideal `[0.5; 8]` is at normalized Euclidean distance `0.5` from every
resource. It cannot express a material preference under that comparison.

[PropertyPoint](src/market/resources.rs) accepts any finite coordinates. The
future target metric must therefore specify coordinate units and whether
out-of-range ideals deliberately represent unattainable desires. Finiteness
alone does not imply coordinates in `[0, 1]`.

**Direction.** Define target coordinates in normalized property units, or
document a different convention consistently. Generate inspectable clusters of
noncentral ideal points. Compare the existing eight-axis lattice with build
variants having four axes of four values or two axes of sixteen values, retaining
all 256 resource identities within each experiment. Changing this interpretation
requires recording the geometry version; an ID's meaning cannot change inside
a saved world.

**Evidence needed.** Histograms of attainable target distances and distinct
rankings of substitutes, plus small examples of which dimensions change a
choice. Report the best reachable match as well as the theoretical ideal:
otherwise unavoidable lattice mismatch may masquerade as poverty or failed
innovation. Use that ceiling as a diagnostic while keeping the satisfaction
function fixed when availability changes.

### D03 — Specify structural matching and the meaning of a dose together

**Critique.** A distance formula quietly decides what manufacturing is for.
If missing structure is cheap, raw units may dominate assembled products. If
padding improves normalization, oversized goods may win. If each target run is
one edit regardless of its quantity, storage encoding changes utility.

**Direction.** Prototype a directional, order-preserving alignment of an ideal
sequence to a good. Charge separately for property mismatch, missing target
positions, and surplus material. Use the target's fixed size as the initial
normalization reference and test the resulting size bias. Treat any proposed
falloff, such as `exp(-distance / tolerance)`, as a hypothesis to compare, not a
settled formula. Reversal and reordering must remain meaningful. Full triangle
inequality is less urgent than consistent consumer rankings.

Use tiny explicitly expanded sequences as a reference oracle. The eventual
implementation must operate on compressed runs or enforce a disclosed search
budget: one legal run can contain `u32::MAX` positions. Adjacent equal ideal runs
must also give the same match whether written separately or together, even
though `Need` currently preserves its supplied representation.

Choose how one whole consumed object reduces intensity and how repeated objects
saturate it. Internal quantity already participates in matching; multiplying
utility by material size again needs justification. Retain the initial rule
that one consumption can benefit multiple needs and measure its consequences.

**Evidence needed.** A published set of tiny comparisons covering padding,
repetition, missing components, run splitting, permutations, giant runs, and
many small objects versus one assembled object. Raw material should sometimes
be useful, while structural assembly should sometimes justify its cost.

### D04 — Generate shared demand without prescribing the winning products

**Critique.** Identical midpoint targets erase differentiation; independent
random long targets may erase shared markets. Adding more similar needs also
increases the value of multifunctional goods under the chosen satisfaction
rule. A changing need count can become an accidental wealth or welfare bonus.

**Direction.** Generate fixed lifetime targets from a small mixture of shared
archetypes, with individual perturbations in ideal points and structure. Keep
archetypes independent of registered goods and the future search algorithm.
Treat targets, tolerances, recovery rates, and importance as separate controls.
Initially vary few of them. Report target overlap and attainable satisfaction
before running the economy.

Define a utility scale and a value of retaining money. One possible purchase
baseline compares the predicted reduction in weighted unmet needs with a
wealth-dependent cost of spending; its coefficients must be reported. Evaluate
marginal benefit after existing inventory and planned consumption, so several
purchases cannot each claim the same unsatisfied need. Buying merely transfers
ownership; consuming separately removes whole inventory units and changes need
intensities. Saving, resale, equipment use, and production remain alternatives.

**Evidence needed.** Cheap imperfect substitutes can beat expensive close
matches; neighboring consumers share some purchases and disagree on others.
Hold a person's total need importance fixed in a diagnostic duplication test
to reveal how much apparent demand comes from representing one desire twice.
Any later satisfaction-capacity rule should respond to an observed problem.

### D05 — Give technology a landscape in which intermediates matter

**Critique.** Material conservation permits every rearrangement of the same
primitive totals. It does not make any rearrangement easy. If manufacturing cost
depends only on final composition or size, intermediate goods may never help:
producing directly from raw resources could always be as cheap. A simulation
could then contain many recipes without a meaningful production network.

**Direction.** Explore a small generative grammar of transformations over
ordered configurations: joining sequences, exposing an internal boundary,
reordering segments, and repeating an assembly operation. Assign irreducible
work to transformations and let exact equipment unlock or cheapen particular
operations. A prepared intermediate should preserve work that otherwise must
be repeated. This grammar constrains feasible processes; it must not become a
handwritten list of named industries.

Separate the simulator's feasibility validator from an agent's costly discovery
procedure. A recipe can conserve matter while lacking any feasible process.
Candidate processes must obey lower bounds or trade-offs among labor, duration,
consumables, equipment occupancy, and losses. Resource substitution can propose
a different product, but a process cannot transmute primitive identities.
Close or rebalance zero-cost transformation cycles as well as individual costs.

**Evidence needed.** Find a tiny case where an intermediate lowers total work
and another where making it is wasteful. An exhaustive search over a small
bounded grammar should expose the available frontier; bounded agents should
sometimes miss it. Do not make discovery easy by granting them that oracle.

### D06 — Resolve the one-output rule before promising splitting or recycling

**Critique.** `register_recipe` conserves inputs against exactly one output
object. It can describe `50 × [(A, 1)] -> 1 × [(A, 50)]`. It cannot describe the
reverse: an input of `[(A, 50)]` contains too much material for one output unit
of `[(A, 1)]`. Thus the statement that splitting needs a recipe is an ontological
rule; it is not yet an executable capability of this schema.

**Direction.** Decide whether to retain one output object and explicitly defer
splitting, or permit a positive integer output multiplicity for a single output
`GoodId`. The latter is a promising small extension because it enables
homogeneous disassembly without introducing arbitrary heterogeneous byproducts.
It remains a proposal requiring a schema decision, not an implied implementation
change. Fractional objects stay forbidden.

Distinguish a recipe's integer transformation ratio from a process's practical
batch. State whether proportionally identical input/output ratios share a
recipe identity, which scale is the smallest legal execution, and which costs
are fixed per batch. Equipment occupancy, workers, duration, and consumables
must not all be assumed to scale in the same way.

**Evidence needed.** If splitting is admitted, a combine/split round trip has
exact primitive accounting and consumes real work. Partial batches, maximum
quantities, and cancellation have defined outcomes. Multi-output waste recovery
can wait until this simpler case is understood.

### D07 — Turn deposits and process losses into a complete physical ledger

**Critique.** A no-gear deposit on `(0, 0)` guarantees material at one place,
not useful material or access for every person. Multi-resource deposits also
leave extraction ambiguous: does a batch select one resource or recover a mix?
The numeric `gear` field has no connection to exact equipment requirements.

**Direction.** Begin with a declared access rule such as presence on the tile,
finite depletion, and reserved extraction slots. Choose batch composition and
what efficiency measures. A one-unit batch at 25% efficiency must not always
produce zero merely because every execution rounds down; use a specified
integer remainder mechanism or seeded stochastic yield. Account for the
remainder physically: unavailable yield either stays in the deposit or enters
an explicit sink, according to the chosen process.

Restrict an initial scenario to no-gear extraction while designing the mapping
from deposit requirements to feasible processes and exact tools. Do not silently
turn an abstract gear level into a complete tool catalogue. Regeneration needs
caps and a named external source of material; its current metadata does not
supply either rule.

Maintain a per-resource balance using mutually exclusive physical states:

```text
initial + external inflows = deposits + stock at rest + jobs + transit + sinks
```

Sinks record consumed or lost material. Ownership is tracked across the physical
states. Reservations are locks on existing stock, not another copy of it.
Consumables and inefficiency inputs must be additional debits even when they
share a `GoodId` with embodied inputs.

**Evidence needed.** Every primitive unit has a destination through extraction,
manufacture, consumption, cancellation, and entity removal. Distinguish equality
of accounting totals from a claim that usable material remains conserved.

### D08 — Make time, labor, equipment, and location genuinely scarce

**Critique.** A worker count and a duration do not allocate anyone's time.
An owned machine cannot support unlimited simultaneous jobs. Giving all goods
the owner's current location would also let a moving owner transport an entire
warehouse instantly.

**Direction.** Use person-producers for the first economy. A job records its
owner, site, exact input reservations, assigned workers, equipment occupancy,
start time, completion time, and cancellation policy. Start atomically, complete
once, and prohibit ordinary zero-duration production initially. Self-employment
uses the same scarce time budget as paid work; its opportunity cost first appears
as forgone feasible activity rather than an invented money transfer.

Represent physical location authoritatively for held, stored, reserved, and
in-transit goods. A first scenario can restrict everything to one site. When
movement arrives, give it duration and a carrying limit measured in primitive
amount or a declared physical capacity measure, not just number of objects.
Inputs, equipment, and workers must coexist at a production site for the
required interval. A transfer of ownership does not move the object.

Add a minimal wage contract before claiming a labor market: specify work time,
payment timing, and who bears cancellation risk. Firms, skills, and equipment
wear can follow after labor allocation and output ownership work.

**Evidence needed.** Two jobs cannot spend the same worker, machine, or input;
movement cannot duplicate location; a faster technique can lose to one with
lower equipment occupancy when capacity is scarce.

### D09 — Close a local cash market and make failed trades informative

**Critique.** Similarity and intensity are not monetary prices. Starting money
is currently 100 integer units per person, so denomination alone can constrain
small transactions. A producer observing no sale cannot tell whether the
problem was poor fit, poverty, no visibility, distance, or missing supply.

**Direction.** Start with local posted offers for exact goods and quantities,
backed by inventory reservations. Buyers choose affordable marginal purchases;
sellers revise prices from observed offers, sales, and unsold stock. Specify
partial fills, expiry, equal-price arbitration, and concurrent buying and
selling. For the first version, trade only where delivery is already possible.
Cash-only person-producers can establish exchange before firms or credit.

Money must transfer among buyers, sellers, and workers. Spawning, death,
saturation, fees, and external grants must have named ledger effects. The
current `Give` adapter clips recipient balances at `u64::MAX`, which can discard
money; pruning also removes balances from `total_money`. Neither behavior can
remain an unexplained monetary event in an economic experiment. Use checked
settlement and reject or reduce an overflowing transfer before debiting it.

Give producers observable market signals with a stated information budget.
Keep private targets available to the analyst without automatically exposing
them to producers. Record why an offer was rejected; distinguish access,
affordability, suitability, and reservation failures in diagnostics.

**Evidence needed.** A closed economy circulates purchasing power across
several cycles. Changing all monetary denominations consistently leaves choices
approximately unchanged except for documented integer rounding. Wealth changes
reconcile to transfers and explicit sources or sinks.

### D10 — Extend snapshot semantics to a complete transaction boundary

**Critique.** Generated heads are staged before actions, which is good causal
hygiene. But `step` already updates needs and prunes entities before a possible
evaluator error. An error is therefore not equivalent to an unchanged tick;
retrying can apply recovery twice. Future jobs and market reservations make this
boundary more consequential.

**Direction.** Specify phases. A candidate sequence is: complete previously due
jobs and arrivals; apply recovery; freeze observations; generate social updates
and action intents; arbitrate claims; commit trades, consumption, job starts,
and departures; apply the chosen survival and estate rules; publish the tick.
Under this proposal a person may respond to current need before a survival
check, unlike today's ordering. New jobs take at least one tick. Proceeds from
this tick should not fund additional intents unless a later funding phase is
explicitly designed.

Use stable entity identities and a recorded seeded or rotating priority rule
for equal claims. Determinism alone does not establish fairness: always choosing
the lowest ID systematically favors early spawns. Once the rules are fixed,
permuting storage rows should not change settlement.

Choose either staged commit with a reliable retry boundary or explicit failed
session/checkpoint recovery. Document which changes become committed if neural
evaluation or validation fails; never automatically retry a partially applied
tick as though nothing happened.

**Evidence needed.** Conflicting buys, consumption, extraction, and production
cannot double-spend. Injected failures, cancellations, and removals preserve
accounting and aligned population/network rows.

### D11 — Define what adapts, how it improves, and what is inherited

**Critique.** “Learn replacements” currently means feeding a candidate network
through a reader and get-to-know network, then installing their outputs. Those
networks start random. Mutation accessors exist, but no reward, selection,
training, reproduction, or credit assignment connects outcomes to better
parameters. Changing connections alone is not evidence of improvement.

**Direction.** First build an experimental adaptation loop around a small,
solvable task. A reasonable starting candidate is population-based mutation and
selection across repeated episodes, retaining private action networks, readers,
and get-to-know networks. Compare it with frozen random controls on held-out
seeds. This is an evaluation proposal, not a requirement that simulated people
reproduce biologically or that all learning be evolutionary.

Separate inherited/base policy parameters from heads installed through personal
acquaintance. Decide whether acquired social connections survive reset,
selection, or reproduction; copying them blindly also copies local slot
meanings. Respect f16 storage when sizing mutations, so proposed changes do not
all round away or overflow.

Choose person-level objectives deliberately. Realized need relief over time,
effort, viability, and retained options are candidates; cash alone would make
wealth accumulation the definition of success. Log wealth and unmet needs
separately even if the controller optimizes one scalar. Delayed production
returns need an evaluation horizon long enough to reward investment.

**Evidence needed.** Adaptation improves a specified outcome over frozen brains
and a simple controller with the same information and actions. Reader and
get-to-know adaptation must have a measurable contribution beyond action-brain
adaptation alone. Report failures across seeds, not just the best lineage.

### D12 — Distinguish social attention from a built-in wealth hierarchy

**Critique.** Every entity currently seeks the globally richest 200 others;
equal wealth is broken by `EntityId`. With equal starting balances, early IDs
therefore attract attention without demonstrating competence. At populations
above 201, the graph is strongly concentrated by construction. At populations
of 201 or fewer, everyone can fit, hiding this effect.

The observation boundary is also unusual: own aggregate resources, global
deposited material, known people's money, and a candidate's full action network.
Reading policy weights does not reveal its current private observations or
prove knowledge of its motives. Retained acquaintances are not reread just
because their brains changed through other slot replacements.

**Direction.** Keep the current ranking as an explicit baseline. Test alternative
candidate-selection rules only as named ablations within the fixed 200-slot
architecture: local encounters, observed productive activity, and a mixture of
familiar contacts with bounded exploration. Define which facts are public and
which require an encounter; global technology knowledge does not settle this.

Decide whether social heads are first impressions or maintained estimates. If
maintained, schedule refreshes by age, prediction error, or brain revision under
a fixed attention budget. Preserve the stable snapshot when several people
reread one another; do not iterate to an implicit social fixed point mid-tick.

**Evidence needed.** Compare attention concentration with wealth, production,
and useful cooperation. A rich but unproductive agent and a cash-poor producer
should expose what the observation rule rewards. Test at least one population
above the slot threshold, and report acquaintance turnover and stale readings.

### D13 — Test whether the reader preserves useful behavioral distinctions

**Critique.** Hidden-symmetry invariance is necessary for the intended reader,
but pooling can also collapse different policies. The middle layers contribute
only weight and bias means. Reading every parameter does not preserve every
relationship among parameters. Moreover, each observer applies only affine
transforms before averaging: in exact arithmetic, `mean(a*x+b) = a*mean(x)+b`.
Private affines cannot recover information that those means discarded.

There is another coordinate question: a candidate's social slot 17 refers to
that candidate's acquaintance, not the observer's slot 17. Hidden-neuron
invariance does not resolve social input/output meanings across people.

**Direction.** Pair symmetry tests with discrimination tests. Construct
functionally equivalent networks, clearly different policies, and policies with
similar pooled summaries but different responses on the same legal observations.
Measure held-out behavior prediction and subsequent economic decisions. Test
consistent relabeling of social slots, occupants, inputs, and target heads;
decide what additional slot context or alignment the reader requires.

If the current pooling fails these tests, investigate a small richer statistic
or bounded behavioral probes before a wholesale reader replacement. Probes must
use observations legitimately available in the chosen experiment. Existing
Monomial invariances remain acceptance requirements.

**Evidence needed.** The reader distinguishes economically different behavior
while ignoring equivalent parameterizations. Compare against money-only,
behavior-history, and shuffled-reading controls under comparable information
and compute budgets. Call a useful result prediction or coordination until a
stronger interpretation is supported.

### D14 — Connect an unbounded economy to a bounded action interface

**Critique.** Enumerating one output per good, price, quantity, recipe, and
location cannot scale with invention. Conversely, an adapter that selects the
best purchase or manufacturing plan on an agent's behalf can conceal all the
intelligence outside its private network. Current aggregate inventory inputs
also hide which materials or useful goods a person owns. Neither need
intensities nor product configurations reach the current action inputs.

**Direction.** Explore bounded menus of legal economic intents generated from
local observations. Candidate records can include a proposed good or job,
price or resource cost, duration, location, need-match information available to
that agent, and the relevant social slot. Reserve some candidate budget for
exploration, so retrieval does not permanently erase unfamiliar products.

Define who generates parameters, who ranks candidates, and how selected intents
map into the action network and generated social heads. Preserving the current
fixed network shape constrains this encoding; any proposed shape or semantic
change needs a coordinated schema decision across constants, readers,
get-to-know outputs, connection installation, and saved brains. Do not casually
repurpose social slots as product IDs.

The environment may reject illegal actions and enforce reservations. Economic
choices should remain attributable to the controller. A diagnostic heuristic
must be labeled and evaluated separately even when it uses the same executor.

**Evidence needed.** Candidate recall on a tiny exhaustively enumerable market,
decisions explained from actual observations, and a stable interface when new
`GoodId`s appear. Report how much performance comes from retrieval versus policy.

### D15 — Make invention costly, reachable, and capable of surprising us

**Critique.** An enormous configuration space is not itself innovation.
Unconstrained random search mostly creates irrelevant designs; direct access to
all private targets would make successful discovery nearly automatic. With
global technological knowledge, discovery also benefits competitors immediately,
so the private return to experimentation needs investigation.

**Direction.** Keep unregistered candidates temporary. Distinguish a proposed
design, a conserved recipe, a feasible process, a process reachable with current
stocks, a manufactured prototype, and a commercially useful product. Current
registration already permits goods without recipes and recipes without
processes; those states are legitimate, but they must be labeled correctly.
Give search a stated labor, material, or evaluation budget.

Explore mutations around actually used products, failed orders, production
bottlenecks, and reusable subsequences. Allow cost-reducing imperfect substitutes,
not only closer matches. Track which signal caused a candidate to be tried.
Separate product search from recipe search and process improvement, and compare
them under equal budgets. Temporary candidate indexes can prune computation;
permanent IDs must never be recycled or make identity depend on a hash score.

Check reachability using consumed inputs, durable equipment, locations, and
quantities together. Permit useful cycles when seed stocks make them executable;
a topological ban would exclude legitimate maintenance or recycling. Apparent
graph connectivity is not enough when two paths compete for the same inventory.

**Evidence needed.** An affordable imperfect invention wins, a technically valid
but unbootstrappable invention remains unused, and a useful discovery sometimes
benefits other producers. Report successful use and welfare gain per search
budget, not just growth of the permanent registry.

### D16 — Budget computation around the actual social workload

**Critique.** Current private action and get-to-know networks contain 53,114 f16
parameters per entity, or 106,228 bytes before other state. At 50,000 people
that is about 5.31 GB of network parameters alone. Initial acquaintance setup can
require 10 million readings and generated bundles. The 72 f32 values in each
bundle alone total 2.88 GB, excluding `Option` overhead, replacement plans,
reader scratch space, device copies, and activations. Bounded GPU evaluation
does not bound all host-side work or memory.

**Direction.** Measure initial setup, steady ticks, and high acquaintance
turnover separately. Run small economic experiments before using population
size as a quality signal. Record time spent in reading, get-to-know evaluation,
action evaluation, candidate retrieval, and economic settlement.

A specific optimization deserves investigation: the current reader's
canonicalization depends on the observed brain, not the observer. Its subsequent
affine pooling can be factored into shared canonical summaries plus private
affines. Cache summaries once per observed brain revision and frozen snapshot;
then apply observer-specific transforms. This follows from the current formula,
but rearranging floating-point operations needs numerical and decision-level
comparison. A future richer reader may not permit the same factorization.

Keep economic caches keyed by stable designs and metric versions, with bounded
active indexes and archival retention of permanent identities. Search limits
and approximation quality belong in experiment records. Streaming or recomputing
pending heads is another option only if every reader still observes the same
unchanged brain pool.

**Evidence needed.** Peak host and device memory, first-tick cost, steady-state
cost, and behavior under a turnover spike. CPU/GPU agreement must include chosen
actions near tied logits; small numerical discrepancies can produce different
long-run trajectories. The existing GPU comparison tests are explicitly ignored
unless a Vulkan device and loader are available.

## A sequence of experiments with stopping conditions

Advance when each experiment explains its result. Failure is useful evidence
about a rule; adding realism is not a substitute for understanding the failure.

| Experiment | Minimal setup and intervention | Evidence required before expanding |
| --- | --- | --- |
| **1. Geometry and structure** | A few explicit resources, targets, and products; exhaustive tiny alignments; padding and run-encoding adversaries. | Sensible substitution rankings, structural assembly sometimes worth doing, equivalent encodings agree, large legal quantities handled safely. |
| **2. A complete physical loop** | One site, a few people, declared survival preset, no-gear extraction, one feasible assembly path, whole-object consumption. | Repeated need relief, exact material accounting, occupied labor/equipment, and no reliance on hidden grants. |
| **3. Useful exchange** | Two different fixed need profiles or production opportunities, local cash offers, then one paid labor task. | Both parties can benefit relative to feasible self-production; balances reconcile; failed trades have interpretable reasons. |
| **4. Invention and capital** | Equal search budgets, a reusable intermediate and optional equipment path, then change input scarcity. | Agents discover a useful alternative; some investment pays back and some does not; availability and capacity matter. |
| **5. Social contribution** | The same economy with adaptive and frozen controllers, correct and shuffled readings, retained and refreshed impressions. | Reader-dependent improvement survives held-out seeds and fair controls; costs and information privileges are reported. |
| **6. Geography and scarce attention** | Two separated sites with complementary resources, costly transport, and a population above 201. | Local scarcity affects choices; delivery cannot teleport; attention effects can be separated from the richest-person rule. |

For every experiment save the configuration, code revision and working-tree
changes, world/population/search seeds, geometry and metric versions, tick
semantics, controller variant, and backend. Separate deterministic random streams
by subsystem and stable identities where possible; adding a world-generation
draw should not silently change someone's initial brain. Report distributions
across seeds and parameter neighborhoods. A striking trajectory is a case study,
not yet evidence of a robust mechanism.

The first useful dashboard should answer: why this consumption, why this offer,
why this process, why this failed job, and why this acquaintance? Show unmet
needs, realized satisfaction, primitive stocks and sinks, occupied capacity,
failed requests, search cost, and active products alongside money. A treasury
balance or count of `GoodId`s alone is a poor measure of economic success.

## High-potential directions tied to this model

These are research hypotheses enabled by the foundations, not promises that the
current scaffold already produces the phenomena.

**Reusable intermediates as an emergent technology vocabulary.** Let the
transformation grammar reward reusing prepared sequences. Ask whether a small
set of intermediates becomes useful across unrelated final products, without
naming those intermediates as special parts. Remove one widely used design in
a controlled replay and measure lost feasible production, substitution, and
recovery time. Compare with removing the same primitive quantity elsewhere to
distinguish structural dependence from simple material scarcity.

**Equipment that creates a new set of opportunities.** A capital good is
interesting when it makes many otherwise impractical transformations viable.
Measure the additional reachable processes and useful products created by one
piece of equipment, under the same labor and material budget. This could reveal
why a tool acquires value before its own direct consumption value is significant.
Avoid rewarding the number of syntactically different recipes it unlocks.

**Social connections as productive investment.** Use bounded refresh and
encounter budgets to ask whether an acquaintance saves failed transactions,
reveals complementary production, or improves demand prediction. Compare its
benefit with the cost of acquiring and maintaining the connection. This would
give the reader architecture an economic role beyond transferring money toward
already visible wealth.

**Discovery as a shared benefit with a private cost.** Retain global knowledge
and let one producer pay for a prototype that others can use. Test whether early
sales, installed equipment, or production experience provide enough private
return to sustain discovery. If invention stalls while society would benefit,
that is a result worth examining before adding patents, subsidies, or secrecy.

**Regional styles with fixed tastes.** Correlate initial need archetypes within
regions while generating material abundance independently. Vary transport cost
and examine whether similar desires produce different locally viable products,
and whether trade converges toward common designs. Targets remain fixed;
product availability, scarcity, and coordination supply the dynamics.

**Competing production histories for the same physical good.** Different
recipes and processes can reach one exact configuration. First study whether
reliability, delivery, and cost produce different seller outcomes without
changing the good's identity. Later attach explicit batch provenance for
reputation or ethical preferences. This can separate physical value from the
value of knowing how and by whom something was made.

**Resilience as an outcome of invention.** After a stable baseline, remove a
material source, machine, or important producer in paired seeded runs. Compare
stored inventory, alternative recipes, substitute products, and new social
contacts as recovery mechanisms. Track unmet needs during recovery, not merely
the eventual return of total output. Shocks should reveal existing dependencies,
not supply the only interesting dynamics in the simulation.

## Keep these questions visible, but sequence them deliberately

The earlier backlog contained useful ideas that should survive consolidation.
The gate below says when a topic becomes informative, rather than dismissing it
as permanently optional.

| Topic | Gate and design constraint |
| --- | --- |
| Context-dependent property weights and need tolerances | First understand the common metric. Keep specificity separate from importance and test normalization effects. |
| Durable goods, rental, maintenance, wear, and replacement | First establish equipment occupancy and disposable consumption. Recurring service has capacity; it cannot grant unlimited satisfaction merely through ownership. |
| Byproducts, recycling, and ecological regeneration | First settle output multiplicity and primitive sinks. Regeneration needs capacities and explicit sources; recovery always requires a feasible process. |
| Storage, spoilage, defects, and physical degradation | First establish locations and inventories. A changed physical configuration is another `GoodId`; do not hide physical quality variation inside fungible units. |
| Transport industries, infrastructure, terrain, and land rights | First demonstrate scarcity with simple costly movement. Sites and rights are separate assets, not resource configurations. |
| Firms, cooperatives, skills, scale economies, and learning by doing | First close person-production and wages. Represent setup costs or acquired capability explicitly; avoid unexplained productivity bonuses. |
| Credit, investment, default, contracts, services, and multiple currencies | First reconcile cash, ownership, and long-lived jobs. Financial and service claims need their own semantics rather than pretending to be physical goods. |
| Harm, health, births, inheritance, and organizational continuity | Resolve current survival immediately under D01. Add richer demography after sustained supply and estate accounting; specify ownership before removing an entity. |
| Local technological knowledge, reputation, brands, and provenance | Start from global technology knowledge. Treat local knowledge as an explicit later experiment; keep physical identity separate from history and seller identity. |
| Satisfaction allocation and evolving tolerances | Measure the chosen independent multi-need benefit first. Any change must leave lifetime targets fixed and be distinguishable from a change of utility scale. |
| Alternative market protocols and equipment compatibility | Compare against one working market. Similar equipment requires another feasible process or an explicit capability rule, never automatic consumer-distance substitution. |
| Compact preference seeds, archives, saved worlds, and ID widths | Add storage optimizations after profiling. Preserve target determinism and permanent IDs; checkpoint formats must record rule and geometry versions. |
| Names, interpretation tools, and experiment presets | Basic causal diagnostics and reproducible small scenarios are essential now. Rich labels and polished sweep tools follow; names must not determine behavior. |

A useful decision record for each issue is short: the chosen rule, the rejected
alternative, the smallest example on which they differ, the evidence that would
change the choice, and the rule's version in saved experiments. This will make
future unresolved notes a record of what the model has taught us as well as what
we still need to build.
