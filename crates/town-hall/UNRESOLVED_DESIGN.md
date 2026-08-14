# Town Hall — unresolved design issues

This document collects the unresolved questions raised in the Town Hall design
discussion and the additional boundaries exposed while implementing the current
economic scaffold.

The categories are implementation priorities, not judgments about how
interesting a feature is:

- **Absolutely essential** means that a functioning version of the intended
  generative economy needs an explicit answer, even if the first answer is very
  simple.
- **High potential** means that the core simulation can run without the feature,
  but it could materially change the kinds of economies that emerge.
- **Nice to have** means that the feature may add texture, realism, usability, or
  efficiency, but is unlikely to prove the central premise by itself.

Items marked **Previously identified** come directly from the design
specifications. Items marked **Newly identified** arose from reviewing and
implementing the scaffold. A mixed label means the original question exposed an
additional implementation boundary.

Nothing here reopens the settled ontology: primitive resources remain fixed
identities in a property space; goods remain exact ordered configurations with
stable IDs; need targets remain ideal configurations independent of `GoodId`;
recipes remain unordered net-input multisets for an exact output; processes
remain practical implementations of recipes; and the neural/social architecture
remains core design even though its present economic action choices are
temporary. All physical, component, inventory, recipe, and need-target quantities
use one universal indivisible unit represented by `u32`. Each primitive resource
has a canonical one-unit configuration `[(A, 1)]`. Inventory multiplicity never
changes configuration identity: `50 × [(A, 1)]` and `1 × [(A, 50)]` are distinct
goods even though conservation accounting finds the same primitive material in
both. Combining, splitting, or rearranging them requires a recipe and a feasible
process.

## Absolutely essential

1. **Define valid configuration boundaries.** — **Previously identified, separated**

   A `u32` permanently settles the numeric representation and makes negative and
   fractional quantities impossible, but zero is still representable. The model
   must decide whether a component such as `(A, 0)` is rejected and whether an
   empty ordered configuration is a valid `Good`. The likely rule remains that
   every component quantity is positive and every good contains at least one
   component, because zero-sized structure complicates canonical identity without
   representing physical matter.

   Need targets require the same explicit boundary decision. Whatever rule is
   chosen should be enforced when configurations enter their canonical registries
   so invalid variants cannot receive permanent IDs. Repeated positive components
   remain valid and structurally significant under the settled sequence model.

2. **Define recipe yield and process batching.** — **Previously identified**

   The scaffold currently uses the cautious convention that one recipe
   execution produces one unit of its output good. This needs to be confirmed or
   replaced. If explicit yields or batch sizes are allowed, the model must say
   which requirements scale linearly, which are fixed per batch, whether partial
   batches are legal, and how duration and equipment occupancy scale.

   Batch semantics affect conservation, capacity, profitability, and innovation.
   They also determine whether two otherwise identical transformations with
   different efficient batch sizes are different recipes or different processes.
   A minimal one-output-unit convention is sufficient initially, but it must be
   treated consistently everywhere.

3. **Generate the primitive physical world.** — **Previously identified, expanded**

   Resource identities and their property positions are fixed after world
   generation, but the actual generation rule is still missing. The simulation
   needs to decide how many resources exist, how their points are distributed in
   the configured property space, how deposits are distributed geographically,
   and how abundance varies. Naive uniform random points can behave surprisingly
   in high dimensions, where almost every pair becomes similarly distant, so the
   generator will strongly shape substitutability.

   Deposit generation must also guarantee a viable starting world rather than a
   formally valid but economically unreachable one. Seeds, reproducibility, and
   parameter reporting are necessary so experiments can distinguish economic
   effects from accidental world-generation artifacts.

4. **Design extraction and access to deposits.** — **Previously identified, expanded**

   Extraction is the bridge from unowned world deposits to owned raw goods. A
   minimal model must decide whether deposits deplete, whether extraction has a
   flow limit, and what labor, time, equipment, consumables, and efficiency are
   required. Treating extraction as a specialized process is conceptually clean,
   but it still needs rules for selecting a tile and reducing its deposit.

   A newly exposed boundary is access: who is allowed to extract a deposit? The
   answer could initially be “any entity present on the tile,” but ownership,
   congestion, simultaneous claims, and reservation rules cannot remain
   implicit. Without an extraction rule, manufactured production can never be
   bootstrapped from the physical world.

5. **Specify the exact configuration-distance algorithm.** — **Previously identified**

   This is the central mathematical subsystem. It must compare an ordered need
   target of ideal property points with an ordered good configuration of actual
   resources while accounting for resource distance, quantity distance,
   insertions, deletions, different lengths, and reordering. The design still
   needs to settle symmetry versus directional suitability, penalties for extra
   and missing components, the effect of excess quantity, whether swapping is a
   primitive operation, and the relative weights of structural and material
   differences.

   The result must be normalized enough to compare targets of different sizes
   and cheap enough to evaluate frequently. An intuitively attractive edit
   distance can still create pathological incentives—for example, padding goods
   with cheap components or exploiting normalization—so adversarial examples and
   small visual experiments should guide the formula.

6. **Convert distance into similarity and satisfaction.** — **Previously identified**

   A distance alone does not determine utility. The model needs a bounded
   falloff from distance to similarity, followed by a satisfaction rule that
   combines similarity, inventory amount consumed, quantities inside one good,
   and current need intensity. It must avoid double-counting physical amount: a
   large configuration consumed once and many small units should have explicitly
   chosen—not accidental—relationships.

   Saturation also needs definition. Satisfaction should not normally reduce an
   intensity below its minimum, and consuming twice as much should not always
   yield twice as much value when the need is almost satisfied. The initial rule
   that one good may independently satisfy every matching need remains settled,
   but its quantitative consequences must be measured.

7. **Define need recovery and the simulation time unit.** — **Previously identified, expanded**

   Need recovery is conceptually nonlinear and bounded, but its exact update rule
   and units are unresolved. Recovery rates are meaningless until a tick has a
   temporal interpretation shared with process duration, employment, transport,
   and equipment occupancy. The model must decide whether recovery is continuous
   but integrated per tick, event-based, or a direct discrete recurrence.

   The first implementation can use a simple stable recurrence such as movement
   toward a maximum. It should nevertheless behave consistently under a changed
   tick size and avoid producing demand primarily as a numerical artifact.

8. **Generate needs that create coherent but heterogeneous demand.** — **Previously identified**

   Every person needs target configurations, but fully independent targets in a
   vast configuration space may leave almost no shared demand and therefore no
   viable mass markets. Conversely, identical targets would remove much of the
   intended product differentiation. A baseline generator must choose the number
   and lengths of needs, ideal property points, target quantities, initial
   intensities, and recovery rates.

   Some controlled clustering is likely necessary even if cultural and social
   preference formation are deferred. The generated distribution should be
   inspectable so product success can be traced to understandable demand rather
   than opaque randomness.

9. **Give needs an economic value scale.** — **Previously identified**

    Similarity and intensity are dimensionless, whereas price is denominated in
    money. The simulation needs a bridge such as maximum willingness to pay,
    need importance, or a utility scale per person. Otherwise an intensity of
    `1.0` cannot say whether the person would spend one monetary unit or their
    entire wealth.

    The minimal model may assign the same value scale to every need, but that is
    still an explicit calibration choice. It must also decide whether the value
    of another unit declines as satisfaction accumulates and how the opportunity
    cost of holding money enters the comparison.

10. **Define consumer choice across goods, needs, and time.** — **Previously identified**

    Consumer surplus is a promising principle, but an operational purchasing
    policy is still absent. A person may face several goods that partially
    satisfy overlapping needs, limited money, inventory already owned, future
    recovery, and uncertainty about later prices. Choosing each purchase in
    isolation can yield a poor bundle or spend the budget repeatedly.

    The first policy needs rules for affordability, quantity selection,
    substitution, tie-breaking, and whether consumption happens immediately or
    is planned separately. It should permit a cheap imperfect substitute to beat
    an expensive close match without requiring globally optimal combinatorial
    planning.

11. **Define the disposable consumption lifecycle.** — **Previously identified, expanded**

    Purchase and consumption are correctly separate, but the latter still lacks
    an execution rule. The model must decide when an entity chooses to consume,
    how inventory is decremented, whether partial units can be used, whether the
    material disappears from active state, and how consumption interacts with
    multiple needs. A minimal disposable-good path is necessary even if durable
    ownership is postponed.

    Storage for later use, resale, recipe use, and equipment use must remain
    valid alternatives. Merely buying a good must never silently satisfy a need
    or destroy the object.

12. **Create a technological-feasibility model.** — **Previously identified**

    Material conservation prevents free matter but does not prevent arbitrary
    rearrangement or a process with zero labor, zero time, no equipment, and
    perfect efficiency. The simulation needs a deeper landscape determining
    which recipes and processes can exist and how trade-offs constrain them.

    This system must support mutation without becoming a handcrafted catalogue
    of industries. It may use latent technological properties, transformation
    costs, capability requirements, bounded improvement frontiers, or another
    generative rule. Whatever the mechanism, it must distinguish accounting
    possibility from practical manufacturability and prevent innovation from
    monotonically mutating every cost to zero.

13. **Finish process material accounting.** — **Previously identified**

    The conceptual distinction between embodied recipe inputs, necessary process
    consumables, and extra inputs lost through inefficiency is settled and is
    represented in the scaffold. Their physical destinations are not. The model
    must decide whether consumed fuel and losses disappear, become one or more
    waste goods, or are tracked as primitive-material residue.

    Validation must state which conservation equation applies to a complete
    process execution. It must also prevent the same input unit from being
    counted as both embodied material and an additional consumable. This is
    necessary before costs or inventory changes produced by a process can be
    trusted.

14. **Schedule production and reserve its requirements.** — **Previously identified, expanded**

    A process has workers, duration, equipment, and consumed goods, but there is
    no runtime production job. Starting a job must atomically reserve or remove
    the relevant inputs, occupy its workers and equipment, record its location
    and completion time, and eventually place output there. Cancellation and
    entity removal during a job also need safe behavior.

    The unresolved equipment-concurrency question is part of this boundary. One
    equipment unit should not normally support unlimited simultaneous jobs.
    Without reservations, several decisions made from the same snapshot can
    spend the same inventory or labor multiple times.

15. **Specify production locality.** — **Previously identified**

    The intended interpretation is that inputs, workers, and equipment coexist
    at a production location for the process duration and output appears at that
    location. Exact rules are still needed for entity presence, inventories
    stored at a tile versus carried by an owner, movable equipment, and jobs
    spanning entity movement.

    A minimal rule can assign every owned physical good to its owner's current
    tile, but that assumption has consequences for companies, warehouses, and
    transport. Location must have one authoritative representation to avoid
    goods or people being present in several places at once.

16. **Build a minimal labor allocation and wage loop.** — **Previously identified**

    Processes require people, yet workers cannot currently offer time, choose
    work, be hired, or receive wages. At minimum, the model must decide who may
    provide labor, how much labor time each person has, how producers acquire it,
    and how compensation is determined and transferred. Self-production may be
    enough for the earliest version, but then its opportunity cost still needs
    representation.

    This loop is economically central because labor income supplies consumer
    purchasing power while production competes for scarce time. It must not
    assume elaborate firms, but it cannot leave workers as a free process input.

17. **Choose the minimal producer/organization model.** — **Previously identified**

    The generic `Entity` can represent people and organizations, but their
    behavioral distinction is unresolved. A first economy needs to say whether
    people can own and run production directly, whether companies are required,
    which kinds of entity can have needs or supply labor, and who owns output and
    pays costs.

    The most cautious route may be person-producers first and explicit companies
    later. Even that choice should be stated so ownership, profit, labor, and
    consumption do not acquire inconsistent assumptions in different systems.

18. **Implement a baseline market and price-discovery mechanism.** — **Previously identified**

    Supply and demand are conceptual outcomes, not an executable market. The
    model must choose a minimal mechanism—such as local posted offers, an order
    book, or a periodic auction—through which a buyer discovers a quantity and
    price, money changes hands, and ownership transfers. It needs rules for
    partial fills, price and quantity priority, insufficient funds or inventory,
    and deterministic settlement.

    The market must trade `GoodId` quantities rather than embedded objects and
    must distinguish merely listing a good from transporting or consuming it.
    The baseline need not be realistic in every detail, but prices cannot emerge
    until some concrete protocol exists.

19. **Define market locality and minimally available information.** — **Previously identified**

    Global technological knowledge does not imply perfect market information or
    access. Consumers need some way to learn that a good is available, where it
    is, its configuration, price, and quantity. Producers need some limited
    signal—historical sales, posted prices, failed orders, or aggregate local
    demand—from which to estimate opportunity without reading private need
    targets.

    A minimal information boundary must avoid both extremes: omniscient producers
    who can calculate every person's exact willingness to pay and blind producers
    who can only invent at random. It should also specify whether distant prices
    are visible before transport and whether information itself has delay.

20. **Implement transportation as a real boundary between locations.** — **Previously identified**

    Geography cannot create trade or specialization if goods can move instantly
    and freely. The initial system needs at least a cost, a duration, a capacity
    rule, and an ownership-safe transfer from one location to another. It must
    specify who initiates and pays for movement and whether goods are unavailable
    while in transit.

    Transportation can initially be a simple formula rather than a full
    productive industry. Even so, distance and physical quantity must affect it,
    and transaction settlement must not teleport goods between local markets.

21. **Design innovation search and the invention lifecycle.** — **Previously identified**

    Product, recipe, and process innovation are conceptually distinct, but no
    actor yet generates or evaluates candidates. The model needs mutation
    operators, search effort or cost, candidate-selection rules, and a way to
    estimate profitability under imperfect information. Search must be capable
    of discovering cheap imperfect substitutes rather than merely optimizing
    geometric quality.

    The lifecycle also needs a decision about whether a `Good` design can be
    registered without a recipe, whether a recipe can exist without a feasible
    process, and when globally known technology becomes actually producible.
    Permanent IDs make speculative registration potentially expensive, so the
    boundary between a candidate and a registered discovery matters.

22. **Resolve cyclic input and equipment dependencies.** — **Previously identified**

    Recipe and equipment graphs can contain cycles such as A consuming B while B
    consumes A, or two goods requiring each other as equipment. Conservation
    prevents matter creation but does not determine whether a network can be
    bootstrapped. The model must either forbid cycles, permit them only with
    pre-existing inventory, or analyze reachability from deposits and existing
    equipment.

    The policy must consider both consumed inputs and non-consumed equipment.
    Otherwise a process may appear valid yet be impossible to start in the
    generated world.

23. **Bootstrap a viable economy at tick zero.** — **Previously identified**

    Starting entities, money, deposits, inventories, raw goods, recipes,
    processes, equipment, organizations, and knowledge have not been selected.
    An entirely blank technological state may never discover extraction; an
    overly rich seed economy may predetermine the industrial structure the model
    is supposed to generate.

    The bootstrap should provide the smallest reachable basis from which the
    intended search can expand. It also needs to decide initial need states and
    how people survive or act before the first market clears. Multiple named
    initialization presets may ultimately be useful, but one coherent baseline
    is essential first.

24. **Close the monetary and ownership accounting loop.** — **Previously identified, expanded**

    Money exists, but its initial distribution, issuance, sinks, and sources are
    unresolved. Wages, purchases, producer revenue, and investment must transfer
    existing balances atomically unless an explicit monetary institution creates
    or destroys money. Inventory ownership needs the same discipline, including
    what happens when an entity dies or an organization fails.

    The temporary `Give` action conserves money except at integer saturation, but
    it is not an economic system. A ledger-level invariant and clear failure
    behavior are required before price or wealth results are meaningful.

25. **Replace the temporary economic action vocabulary and observations.** — **Previously identified, expanded**

    The neural/social architecture is settled, but `Conserve`, `Predict`, and
    `Give` are explicitly not. A complete economic decision model must decide
    what actions entities can take—buy, sell, consume, work, produce, invent,
    transport, wait, or combinations—and how quantities, targets, goods, prices,
    and locations are parameterized without exploding the output space.

    The observation side is equally important. Current inputs containing own
    money, total inventory units, total world deposits, and known-person money
    are placeholders. Economic state must be encoded while preserving the fixed
    social-reading and connection machinery. This is the main integration point
    between the settled neural model and the new economy.

26. **Define tick ordering, atomicity, and conflicting decisions.** — **Newly identified**

    Once markets, jobs, movement, consumption, and need recovery exist, their
    order within a synchronous tick can change outcomes. The simulation needs a
    phase model or event scheduler specifying what is observed, what is reserved,
    when prices clear, when jobs complete, when needs recover, and when entities
    may die or be removed.

    Decisions should normally be made from a stable snapshot and resolved without
    iteration-order advantage. Conflicts for the same good, worker, equipment, or
    deposit need deterministic arbitration. This extends the atomic design already
    used by the neural/social tick into the economic state.

27. **Control combinatorial growth without destroying emergence.** — **Previously identified, expanded**

    The space of ordered goods, alternative recipes, processes, and consumer
    comparisons is enormous. Permanent `GoodId`s mean failed inventions can
    accumulate forever, while evaluating every known good against every need or
    every possible mutation is infeasible. The implementation needs candidate
    indexes, caching, locality-sensitive search, bounded exploration, or another
    strategy that keeps runtime and memory finite.

    Limits must be economic or computational policies rather than hidden semantic
    categories. They should not accidentally forbid long supply chains, niche
    products, or nearby substitutes—the phenomena the model is meant to reveal.

28. **Define validation experiments and calibration criteria.** — **Previously identified, expanded**

    “Reasonable” emergence needs measurable tests. The initial experiments should
    examine smooth substitution, quality-versus-price trade-offs, coherent shared
    demand, material conservation, trade under spatial scarcity, useful
    intermediates, capital goods, recipe competition, process improvements, and
    resistance to giant or otherwise exploitative configurations.

    The simulator also needs diagnostics exposing why a consumer bought a good
    and why a producer expected profit. Without these, tuning the distance,
    utility, and search systems risks fitting intuition by guesswork or mistaking
    implementation bugs for emergent economics.

## High potential, but not strictly necessary

1. **Context-dependent resource-property weighting.** — **Previously identified**

   A universal resource distance is sufficient for a first model, but it makes
   two materials equally substitutable in every application. Need-specific
   weights could make dimensions important in different proportions for
   different desires, while process-specific weights could eventually influence
   technological compatibility. This would create materials that are close
   substitutes for one purpose and poor substitutes for another.

   The major risk is adding opaque parameters and computation before the base
   metric is understood. Context weights should therefore arrive only after a
   universal metric has been visualized and validated.

2. **Need-specific width or tolerance.** — **Previously identified**

   Giving every need the same falloff assumes identical specificity. A width
   parameter could distinguish broad desires accepting many configurations from
   narrow desires requiring an unusually close match. This could produce mass
   commodity markets alongside specialized niches even when intensities are
   similar.

   Width must not become a second arbitrary importance scale. Its role should be
   limited to geometry, with economic importance handled separately.

3. **Richer cultural, geographical, and social preference structure.** — **Previously identified**

   A minimal clustered need generator can create coherent demand, but culture
   and geography could make tastes correlate within communities and diverge
   between them. Social interaction could also diffuse or differentiate targets
   over time. This would support regional products, fashion, cultural trade, and
   changing market boundaries.

   Such dynamics have high emergent potential, but fixed generated clusters are
   enough to test the physical and market model first.

4. **Durable consumer goods and service flows.** — **Previously identified**

   The disposable consumption rule cannot represent houses, furniture, or
   personal tools well. Durable goods could remain owned and provide recurring
   satisfaction while available, perhaps with occupancy, capacity, maintenance,
   or eventual degradation. This would create saving, replacement cycles, rental
   opportunities, and household capital.

   It is not required for the first recurring-consumption economy, but excluding
   it permanently would bias nearly all successful consumer goods toward
   disposable analogues.

5. **Heterogeneous need importance and marginal utility of wealth.** — **Previously identified**

   A common willingness-to-pay scale can bootstrap consumer choice. Later,
   person- and need-specific importance would distinguish urgent desires from
   minor preferences, while a wealth-dependent marginal value of money would
   make the same price affect rich and poor people differently. This could
   generate premium markets, budget goods, and distribution-sensitive demand
   more naturally.

   Care is needed to avoid encoding desired economic outcomes directly through
   arbitrary utility coefficients.

6. **Consequences of unmet needs and harmful goods.** — **Previously identified**

   Current needs motivate demand but do not represent biological necessity, and
   similarity is nonnegative. Some needs could eventually affect health,
   productivity, happiness, or survival when left unmet; some configurations
   could actively produce negative utility rather than merely zero satisfaction.
   These rules could connect consumer markets to population and labor dynamics.

   They are not necessary to demonstrate generative products and could create
   brittle death spirals if introduced before supply and purchasing are stable.

7. **A satisfaction budget for multifunctional goods.** — **Previously identified**

   Initially, one consumed good independently satisfies every matching need.
   This intentionally permits multifunctional products, but it may multiply a
   good's value without bound when a person has many nearby needs. A later model
   could allocate a fixed amount of satisfaction among needs, introduce shared
   capacity, or let the consumer choose how the good is used.

   This should be driven by observed exploits rather than changed preemptively,
   because independent satisfaction is simpler and already the chosen initial
   behavior.

8. **Multiple outputs, byproducts, physical waste, and recycling.** — **Previously identified**

   A single primary output with disappearing losses is sufficient initially, but
   useful byproducts can create important cross-industry relationships. A process
   could emit several explicit goods, or waste could itself be a physical good
   that later recipes recover. Recycling and disassembly would then require real
   recipes and feasible processes rather than automatic access to flattened
   material.

   This could produce industrial symbiosis and secondary-resource markets, but it
   significantly complicates profitability allocation and production accounting.

9. **Renewable resources and ecological dynamics.** — **Previously identified**

   The essential extraction model can begin with fixed depleting deposits.
   Regeneration, renewable flows, seasonal variation, and resource damage could
   later create sustainable extraction strategies and long-run scarcity. An
   explicit environment could also receive process waste or other externalities.

   These systems have major long-horizon potential, but they should not obscure
   whether the basic production and trade loops work.

10. **Transportation as an endogenous industry.** — **Previously identified**

    After a simple movement cost proves geography meaningful, transportation
    could itself require labor, time, vehicles, consumables, route capacity, and
    infrastructure. Vehicles would then emerge as equipment goods and transport
    firms could specialize like any other producer. Infrastructure investment
    could reshape comparative advantage and regional clustering.

    This is much richer than a distance multiplier, but the simple boundary is
    enough to establish trade first.

11. **Equipment degradation, maintenance, and replacement.** — **Previously identified**

    Infinite-lived equipment creates capital goods and capacity but omits a major
    source of recurring industrial demand. Wear could depend on use or time,
    maintenance could consume goods and labor, and failed equipment could become
    waste or material for disassembly. Different processes might trade higher
    capital wear for speed or labor savings.

    This feature is economically powerful but can wait until equipment occupancy
    and production scheduling are correct.

12. **Worker skills and human-capital differences.** — **Previously identified**

    Counting workers treats all labor as interchangeable. Skills, learning, and
    process-specific productivity could generate wage differences,
    specialization, training, migration, and technological bottlenecks. A process
    might require a capability distribution rather than merely a head count.

    Generic labor is a valid first abstraction; skills should be introduced only
    if the labor market behaves sensibly without them.

13. **Explicit firms and organizational behavior.** — **Previously identified**

    Person-producers can close the minimal economic loop, but companies could own
    inventory and equipment, hire people, retain profit, invest, enter, fail, and
    coordinate multi-stage production. Their objective functions and governance
    need not mirror human needs. Organizations could also represent cooperatives
    or public institutions using the same generic entity substrate.

    This greatly expands industrial structure, yet it is safer to add after
    individual production, labor, and ownership are unambiguous.

14. **Decentralized technological knowledge.** — **Previously identified**

    Global knowledge is the settled initial rule. Later, entity-local knowledge
    of goods, recipes, and processes could enable discovery diffusion, imitation,
    secrecy, trade secrets, research specialization, and regional technological
    divergence. Knowledge acquisition could itself consume time or depend on
    social connections.

    It has high potential, but global knowledge removes a major confounder while
    the feasibility and innovation systems are being validated.

15. **Production provenance, ethical preferences, brands, and reputation.** — **Previously identified**

    Units sharing a `GoodId` are physically fungible, but batches may eventually
    carry recipe, process, location, labor, or environmental provenance. Consumers
    could value that metadata without pretending the physical configuration is a
    different good. Brands and reputation could summarize expected provenance or
    seller reliability under imperfect information.

    This supports rich differentiation, but it introduces a second desirability
    channel that should not mask weaknesses in configuration-based preference.

16. **Credit, investment, insolvency, and capital finance.** — **Previously identified**

    Production requiring expensive equipment may be impossible when actors can
    spend only current money. Loans, equity, retained earnings, and investment
    decisions could fund capital-intensive processes and create financial
    networks. Defaults and liquidation would then affect ownership and production
    capacity.

    These mechanisms are potentially transformative, but a cash-only economy is
    sufficient to validate prices, labor, and production first.

17. **Economies of scale, fixed costs, and learning by doing.** — **Newly identified**

    Linear per-unit recipes and processes may produce specialization, but they do
    not naturally represent setup costs, scale-efficient batches, or accumulated
    operational experience. Fixed setup requirements and process improvement
    through repeated use could create firm growth, market concentration, and path
    dependence without hard-coded industries.

    These should be distinguished from arbitrary producer bonuses and added only
    after base process costs and batching are stable.

18. **Storage capacity, spoilage, and inventory carrying costs.** — **Previously identified, expanded**

    Goods currently remain in inventory indefinitely and without space or cost.
    Storage requirements, decay, spoilage, or quality degradation could constrain
    stockpiling and create warehousing, inventory cycles, and geographic
    bottlenecks. Physical degradation should normally result in another exact
    `GoodId`, not hidden quality variation within one ID.

    These effects are valuable for dynamics but unnecessary for the first static
    physical ownership model.

19. **Land, sites, and deposit rights as economic assets.** — **Newly identified**

    A minimal extraction rule can grant simple access by presence. A richer model
    could make tiles, extraction rights, factory sites, or transport corridors
    ownable and tradable outside the physical `Good` system. Rent and location
    scarcity could then influence industrial clustering independently of raw
    material prices.

    This has substantial potential but belongs to the broader ownership system,
    not to `GoodId`.

20. **Population dynamics, inheritance, and organization continuity.** — **Newly identified**

    The broader simulation permits entity survival and death, but a full economy
    may eventually need births, inheritance of money and goods, company
    continuity, estate handling, and migration. These affect long-run wealth
    distributions and preservation of capital or knowledge.

    They are not needed to test the core economic ontology and should follow a
    reliable ownership and monetary ledger.

## Nice to have or likely marginal initially

1. **Procedural preference seeds as a memory optimization.** — **Previously identified**

   Persistent needs could be regenerated from a compact per-person seed instead
   of storing every target. This may matter at very large populations, provided
   generated needs remain deterministic and independent of product identity.
   Geometry—not hashing—must still determine similarity. It is an implementation
   optimization and should wait until profiling shows need storage is material.

2. **Archival compression for the permanent good registry.** — **Previously identified**

   Stable `GoodId`s must never be reused, even when a product has no inventory,
   demand, production, or known recipe. If failed innovation creates millions of
   entries, cold configurations and historical metadata could be compacted or
   moved to archival storage while retaining lookup and identity. This is useful
   only after search-policy controls and real registry growth are measured.

3. **Resource-specific physical measurement units.** — **Previously identified**

   Different real materials are commonly measured by mass, volume, count, or
   energy. Resource-specific units could add realism, but they would complicate
   conservation and make target quantities harder to compare. A universal
   abstract material scale is strongly preferred initially; specialized units
   should be considered only if a concrete economic phenomenon cannot be
   represented without them.

4. **More sophisticated market protocols.** — **Previously identified**

   Once one baseline mechanism works, bilateral negotiation, continuous order
   books, auctions, posted-price competition, contracts, or hybrid local markets
   could be compared. These alter price dynamics and information demands, but
   implementing several mechanisms before validating one would add surface area
   without resolving the core ontology.

5. **Contracts, shares, debt claims, and other nonphysical tradables.** — **Previously identified**

   The design correctly excludes labor, services, land, debt, shares, and legal
   claims from `Good`. Separate registries could eventually support these assets.
   They may enrich coordination and finance, but most should follow rather than
   precede the cash market, ownership ledger, and optional firm system.

6. **Markets for nonphysical services.** — **Previously identified, expanded**

   Labor is essential, but broader services could be sold without becoming
   physical goods: transport service, research, repair, education, or access to
   equipment. A general contract or service system would avoid forcing them into
   resource configurations. Initially, each can be represented by a dedicated
   action or process relationship if it becomes necessary.

7. **Approximate compatibility for substitute equipment.** — **Previously identified as future**

   Current processes require exact equipment `GoodId`s, and a similar good only
   works through another process. A later feasibility system could generate
   families of compatible equipment or parameterize capabilities geometrically.
   This may reduce process duplication, but automatic similarity must not bypass
   the settled distinction between consumer similarity and technological
   feasibility.

8. **Stochastic manufacturing defects and quality variation.** — **Previously identified, expanded**

   A process currently produces exactly its declared output. Random defects could
   instead produce another registered configuration or waste, preserving the
   rule that physically inferior units cannot hide under the same `GoodId`.
   Reliability would then affect expected cost and perhaps equipment choice. This
   is realistic texture, but deterministic production is much easier to diagnose.

9. **Compact production-history fingerprints.** — **Previously identified**

    Full production trees can become large. If provenance is implemented, batches
    could carry interned lineage records or compact fingerprints linking back to
    recipes, processes, and source batches. This is primarily a storage and query
    concern; it is unnecessary while provenance preferences remain deferred.

10. **Human-readable names and semantic interpretation tools.** — **Previously identified, expanded**

    The simulation must not rely on labels such as wood, chair, or machine, but a
    user interface could assign descriptive names after observing resource
    properties, configurations, and network roles. Visualizing nearest resources,
    need matches, supply chains, and emergent capital goods would make experiments
    much easier to understand without changing simulation behavior.

11. **Richer geography and terrain.** — **Previously identified as implementation-specific**

    The flat rectangular tile grid is adequate initially. Terrain, non-Euclidean
    connections, rivers, borders, elevation, or changing routes could alter
    transport and settlement. These are worthwhile only after simple distance
    already produces meaningful local prices and specialization.

12. **Multiple currencies and exchange markets.** — **Newly identified**

    A single money type is sufficient to close the economic loop. Regional or
    issuer-specific currencies could create exchange rates, monetary scarcity,
    and financial geography, but they introduce major calibration questions with
    little direct benefit to testing generative goods, recipes, and processes.

13. **Taste evolution, fashion, and habituation.** — **Newly identified**

    Need targets are initially persistent while intensities recover. Targets or
    tolerances could later drift through experience, social influence, or
    repeated consumption, producing fashion cycles and endogenous demand change.
    This may be interesting, but stable tastes provide a much clearer environment
    for evaluating whether product innovation works at all.

14. **Stochastic shocks, accidents, and disasters.** — **Newly identified**

    Deposit discoveries, process accidents, weather, transport loss, or sudden
    demand shocks could test resilience and create historical contingency. They
    should be reproducible from the simulation seed and visible in diagnostics.
    Such shocks add drama and robustness testing, but should not compensate for
    weak endogenous dynamics in the base economy.

15. **Serialization, schema migration, and ID-width policy.** — **Newly identified**

    Stable resource, good, recipe, process, and entity identities will eventually
    need a saved-world format. The current numeric widths impose capacity limits,
    and future changes to quantity or property representations will require
    migration. This is important for a long-lived application but can wait while
    the core schemas are intentionally changing quickly.

16. **Experiment presets and parameter-sweep tooling.** — **Previously identified, expanded**

    Resource dimensions, resource counts, map size, needs per person, recovery,
    innovation rates, and transport costs are natural experimental parameters.
    Named presets, batch sweeps, and summary reports would make comparative study
    easier. The immediate priority is defining the underlying rules and a small
    diagnostic scenario; polished experiment orchestration can follow.
