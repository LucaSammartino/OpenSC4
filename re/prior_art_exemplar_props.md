# SC4 simulator tuning exemplar property IDs (extracted)

Source: SC4Devotion wiki 'Exemplar Properties' page (archived copy, https://web.archive.org/web/2024/https://wiki.sc4devotion.com/index.php?title=Exemplar_Properties ; live wiki is behind a Cloudflare challenge). Community-maintained; descriptions are Maxis property-editor strings where available, otherwise community guesses. '(Obs)' = obsolete per Maxis XML.

Known tuning-exemplar TGIs (verified in DLL source): Traffic Simulator = 0x6534284A / 0xE7E2C2DB / 0xC9133286 (sc4-park-and-ride-ordinance); Building Development Simulator = 0x6534284A / 0xE7E2C2DB / 0xE8DA7677 (sc4-no-kick-out). Group 0xE7E2C2DB appears to be the simulator-tuning group.

## Simulators

| Section | Property ID | Type | Name | Description |
|---|---|---|---|---|
| Main Simulator | 0x89590e6d | Uint32 | Simulator Speeds | Milliseconds per day for slow, medium, fast |
| Main Simulator | 0x897e709f | Float32 | 24 Hour Clock speed (time magnification) | Specifies a multiplier from real time to the 24 hour clock's time. |
| Main Simulator | 0x09b73421 | Uint32 | k24HourClockNightBeginProperty | Specifies the time (0-23) that night time graphically begins. |
| Main Simulator | 0x09b73422 | Uint32 | k24HourClockNightEndProperty | Specifies the time (0-23) that night time graphically ends. |
| Main Simulator | 0x89590e6e | Float32 | kAnimationTimeDilationProperty | Time dilation factor to apply to animations based on sim speed (paused,slow,med,fast,emergency/drive) |
| Land Value Simulator | 0x47e2c300 | Uint8 | Land Value Intrinsic Min/Max | Minimum > 0, Maximum |
| Land Value Simulator | 0x47e2c301 | Uint8 | Land Value Wealth Boundaries | Low/Medium boundary, Medium/High boundary |
| Land Value Simulator | 0x47e2c310 | Uint8 | Land Value Water Effect | Effect of water on land value: size, radius |
| Land Value Simulator | 0x47e2c320 | Float32 | Land Value Altitude Effect | Maps altitude to land value |
| Land Value Simulator | 0x47e2c330 | Uint32 | Land Value Desirability IDs | Growth Developer IDs (see Effects) |
| Land Value Simulator | 0x47e2c331 | Float32 | Land Value Desirability Factors | Size of effects (see IDs) |
| Land Value Simulator | 0x47e2c332 | Float32 | Land Value Desirability Curve | Maps desirability to land value |
| Building Development Simulator | 0x47e2c380 | Uint32 | Construction Resources Min/Max | Amount of capacity that can be under construction at any one time |
| Building Development Simulator | 0x47e2c381 | Float32 | Construction Resources per Resident | Factor applied to population to calculate construction resources |
| Building Development Simulator | 0x47e2c382 | Uint32 | Construction Type Selection Bias | Improves that chance that low-demand types will have opportunity to build |
| Building Development Simulator | 0x47e2c383 | Float32 | Stage Popluation Multiplier | Values > 1 allow big stage buildings sooner. Indexed by difficulty level |
| Building Development Simulator | 0x47e2c400 | Uint32 | Lot Developer Stay New Time | Number of days before a building can distress |
| Building Development Simulator | 0x47e2c401 | Float32 | Lot Developer Occupancy Thresholds | When to abandon, distress, and renew. Values 0-1 |
| Building Development Simulator | 0x47e2c500 | Float32 | Tract Developer Overbuild | Factor applied to desired capacity when growing buildings in a tract |
| Building Development Simulator | 0x47e2c501 | Float32 | Tract Developer Reoccupy Thresholds | Start and Stop percentage of desired capacity that can be fulfilled by abandoned buildings |
| Building Development Simulator | 0x47e2c502 | Float32 | Tract Developer Redevelop Size Threshold | What percentage of the existing capacity must be demanded before we redevelop |
| Building Development Simulator | 0x47e2c510 | Float32 | Tract Developer Demolition Costs | For occupied/abandoned buildings" |
| Building Development Simulator | 0x47e2c511 | Float32 | Tract Developer New Lot Costs | Fictional cost to aggregate a lot/subdivide a lot |
| Building Development Simulator | 0x47e2c530 | Float32 | Tract Developer Occupancy Variation | Building occupancy rate +/- variation from tract rate |
| Building Development Simulator | 0x47e2c531 | Float32 | Tract Developer Min Vacancy Range | When 100% full, subtract a little from the occupancy rate |
| Building Development Simulator | 0x47e2c540 | Bool | Tract Developer Kick Out Lower Wealth | If true, higher wealth buildings can replace non-empty lower wealth buildings |
| Building Development Simulator | 0x47e2c550 | Uint32 | Tract Developer Available Styles | List of available (R and C) building styles |
| Building Development Simulator | 0x47e2c551 | Sint32 | Tract Developer Years Between Styles | How often to change (R and C) building styles |
| Building Development Simulator | 0xe900ea24 | Float32 | kMaxCellAltitudeDeltaForFlatnessProperty | kMaxCellAltitudeDeltaForFlatnessProperty |
| Building Development Simulator | 0xe900ea25 | Float32 | kMaxSlopeForFlatnessProperty | kMaxSlopeForFlatnessProperty |
| Building Development Simulator | 0xe900ea26 | Uint32 | kDefaultLotFoundationTypeProperty | kDefaultLotFoundationTypeProperty |
| Building Development Simulator | 0xe900ea27 | Uint32 | kShouldAttemptLotLevellingProperty | kShouldAttemptLotLevellingProperty |
| Building Development Simulator | 0xe900ea28 | Uint32 | kShouldAttemptInternalLotLevellingProperty | kShouldAttemptInternalLotLevellingProperty |
| Building Development Simulator | 0xe900ea29 | Float32 | kLotAutoLevelInternalAltitudeDeltaProperty | kLotAutoLevelInternalAltitudeDeltaProperty |
| Building Development Simulator | 0xe900ea2a | Float32 | MaxAreaSlopeAllowedForLevellingProperty | MaxAreaSlopeAllowedForLevellingProperty |
| Building Development Simulator | 0xe900ea2b | Uint32 | Lot Developer: Night Update Interval | Update interval in milliseconds for toggling building lights. |
| Building Development Simulator | 0xe900ea2c | Uint32 | Lot Developer: Night Update Count | Number of buildings to toggle lights on per update interval (integer). |
| Building Development Simulator | 0x2a2d7824 | Float32 | Lot Developer: Cost Multiplier vs lot slope | Cost to plop a lot is building + area x this response curve |
| Building Development Simulator | 0x2ab845e7 | Uint32 | Lot Developer: Cost to Demolish Destroyed Tile | Cost to demolish a tile on a lot that has been destroyed |
| Demand Simulator | 0x4a1f38b5 | Float32 | Demand: Neutral Tax Rate vs. Population | What is the highest tax rate you can set before demand suffers? |
| Demand Simulator | 0x4a1f38b6 | Float32 | Demand: Tax Modifier vs. Rate Variance | How much is demand impacted by tax rates above or below neutral? |
| Demand Simulator | 0x4a1f38b7 | Uint32 | Demand: Days of Tax Penalty | How long a high tax rate depresses demand (even after it is lowered) |
| Demand Simulator | 0x4a1f38c5 | Float32 | Demand: Commercical Cap Relief for Connections | Relief for first connection (more have diminishing returns) |
| Demand Simulator | 0x4a1f38c6 | Float32 | Demand: Commercical Cap Relief for Airports | Relief per connected Co population |
| Demand Simulator | 0x4a1f38c7 | Float32 | Demand: Commercical Cap Relief for Seaports | Relief per industrial trip |
| Demand Simulator | 0x4a1f38c8 | Float32 | Demand: Industrial Cap Relief for Trips | Relief per neighbor trip |
| Demand Simulator | 0x4a1f38c9 | Float32 | Demand: Industrial Cap Relief for Airports | Relief per connected Co population |
| Demand Simulator | 0x4a1f38ca | Float32 | Demand: Industrial Cap Relief for Seaports | Relief per industrial trip |
| Demand Simulator | 0x8a63f2a1 | Uint32 | Average Salary for Demographic | The average salary for each of the three wealth levels and four education levels (12 total) |
| National Economy Simulator | 0x4a1f629f | Uint32 | Nation: Demand ID list | List of IDs corresponding to Demand Modifiers |
| National Economy Simulator | 0x4a1f62a0 | Float32 | Nation: Default Demand Modifier vs. Month | Demand boost over time |
| National Economy Simulator | 0x4a1f62a1 | Float32 | Nation: Demand Modifier 1 vs. Month | Demand boost over time |
| National Economy Simulator | 0x4a1f62a2 | Float32 | Nation: Demand Modifier 2 vs. Month | Demand boost over time |
| National Economy Simulator | 0x4a1f62a3 | Float32 | Nation: Demand Modifier 3 vs. Month | Demand boost over time |
| National Economy Simulator | 0x4a1f62a4 | Float32 | Nation: Demand Modifier 4 vs. Month | Demand boost over time |
| National Economy Simulator | 0x4a1f62a5 | Float32 | Nation: Demand Modifier 5 vs. Month | Demand boost over time |
| National Economy Simulator | 0x4a1f62a6 | Float32 | Nation: Demand Modifier 6 vs. Month | Demand boost over time |
| National Economy Simulator | 0x4a1f62a7 | Float32 | Nation: Demand Modifier 7 vs. Month | Demand boost over time |
| National Economy Simulator | 0x4a1f62a8 | Float32 | Nation: Demand Modifier 8 vs. Month | Demand boost over time |
| National Economy Simulator | 0x4a1f62a9 | Float32 | Nation: Demand Modifier 9 vs. Month | Demand boost over time |
| National Economy Simulator | 0x4a1f62b0 | Float32 | Nation: Demand Modifier 10 vs. Month | Demand boost over time |
| National Economy Simulator | 0x4a1f62b1 | Float32 | Nation: Demand Modifier 11 vs. Month | Demand boost over time |
| National Economy Simulator | 0x4a1f62b2 | Float32 | Nation: Demand Modifier 12 vs. Month | Demand boost over time |
| National Economy Simulator | 0x4a1f6300 | Float32 | Nation: Difficulty Demand Modifier | Demand boost, indexed by difficulty |
| Utility Simulator | 0x6911e2df | Float32 | Power Funding To Efficiency Curve | Maps difference between actual % funding and % ideal funding to % of a utility output |
| Utility Simulator | 0x0911e2e6 | Float32 | Utility Strike Response Curve | Maps difference between actual % funding and % ideal funding to % chance of a utility strike |
| Utility Simulator | 0x29121e8b | Float32 | Power Min Acceptable Funding % | Minimum acceptable percentage for Utility department funding |
| Utility Simulator | 0x09122027 | Float32 | Utility Strike Efficiency % | Efficiency of utility buildings will drop to this % during a strike |
| Utility Simulator | 0x2912202e | Uint32 | Utility Strike Duration | Maximum duration of a Utility strike, in months |
| Utility Simulator | 0x49122036 | Float32 | Utility Min Funding to End Strike | Minimum funding percentage that must be restored to end a utility strike |
| Utility Simulator | 0xe911e2d1 | Float32 | Power max allowed funding | Maximum allowed funding percentage for utilities |
| Utility Simulator | 0xa911e2f1 | Uint32 | Power radiation radius | Number of cells surrounding buildings and zones that will also be powered |
| Utility Simulator | 0xe911e2fc | Float32 | Power plant max production ratio | Maximum allowed percentage for power plant production |
| Utility Simulator | 0x49134bd4 | Uint32 | Max garbage value for UI | Maximum number of garbage units reported to the UI, must be less than 32767 |
| Utility Simulator | 0x49134bdd | Uint32 | Max air pollution value for UI | Maximum air pollution value reported to the UI, must be less than 32767 |
| Utility Simulator | 0x09134be2 | Uint32 | Max water pollution value for UI | Maximum water pollution value reported to the UI, must be less than 32767 |
| Utility Simulator | 0x89135044 | Uint32 | Max radiation pollution value for UI | Maximum radiaiton pollution value reported to the UI, must be less than 32767 |
| Utility Simulator | 0x0913519b | Uint32 | Garbage Pollution Threshold | When a cell has this much garbage, it's considered polluted |
| Utility Simulator | 0xa91351a6 | Uint32 | Air Pollution Threshold | When a cell has this much air pollution, it's considered polluted |
| Utility Simulator | 0x091351ae | Uint32 | Water Pollution Threshold | When a cell has this much water pollution, it's considered polluted |
| Utility Simulator | 0x291351b5 | Uint32 | Radiation Pollution Threshold | When a cell has this much radiation, it's considered polluted |
| Utility Simulator | 0x09135479 | Uint32 | Garbage decompostion amount | Number of garbage units that decompose (i.e., disappear) each month |
| Utility Simulator | 0x09135482 | Float32 | Garbage UI scaling factor | Scaling factor to convert simulator garbage units to whatever units the UI displays (tons, Kgs) |
| Utility Simulator | 0x68f5436a | Uint32 | Water pipe effect radius | Number of cells surrounding water pipes that will also be watered |
| Utility Simulator | 0xaa161f81 | Uint32 | Max acceptable pumped water pollution level | Water pumps will not function if water pollution higher than this level |
| Utility Simulator | 0x68f5436b | Uint32 | Fresh water pump reach | Number of cells away from fresh water a water pump can be placed |
| Utility Simulator | 0x68f5436c | Uint32 | Salt water pump reach | Number of cells away from salt water a water pump can be placed |
| Utility Simulator | 0x68f5436d | Uint32 | Fresh water distance loss | Percentace by which water production is reduced when not within range of fresh water source |
| Utility Simulator | 0x69501944 | Float32 | Traffic air pollution factor | Affects how traffic density equates to air pollution |
| Utility Simulator | 0x098b25c8 | Uint32 | Max cells between power poles | The maximum number of cells allowed between power poles |
| Utility Simulator | 0x098b27d0 | Float32 | Minimum power line height | The minimum distance allowed between a power line and the terrain |
| Utility Simulator | 0xe9a18d3f | Float32 | Power line control max horiz. distance | Max horizontal distance from the center of the line at which the control points are placed |
| Utility Simulator | 0x49a1e8c4 | Float32 | Power line control horiz. % | Percent of line length at which to place control points, horizontally |
| Utility Simulator | 0x89a18fc4 | Float32 | Power line control max vertical distance | Max vertical distance from the center of the line at which the control points are placed |
| Utility Simulator | 0xe9a1e8d4 | Float32 | Power line control vertical % | Percent of line length at which to place control points, vertically |
| Utility Simulator | 0x49a195c0 | Float32 | Power line curve distance between points | Distance between the points generated for power line curves |
| Utility Simulator | 0x4a5f5953 | Uint32 | Power pole placement cost | Cost to place a power pole using the power line tool |
| Utility Simulator | 0x0a5f596b | Float32 | Power line placement cost | Cost of placing each cell of wire |
| Utility Simulator | 0x89a1e8fb | Uint32 | Minimum number of curve points | Minimum number of points to generate on power line Bezier curve |
| Utility Simulator | 0x69a437f6 | Float32 | Garbage Prop Request Curve | Shares of garbage props requested vs. garbage percentage of max |
| Utility Simulator | 0x0a10c772 | Float32 | Water Pollution Effect Strength Curve | Water Pollution Average Level to Water Pollution Decal Alpha curve |
| Utility Simulator | 0x0a10c773 | Float32 | Air Pollution Effect Strength Curve | Air Pollution Average Level to Air Pollution Particle Emission Rage curve |
| Utility Simulator | 0x0a10c774 | Float32 | Radiation Pollution Effect Strength Curve | Radiation Pollution Average Level to Radiation Effect Strength curve |
| Utility Simulator | 0x0a10c775 | Float32 | Garbage Pollution Effect Strength Curve | Radiation Pollution Average Level to Radiation Effect Strength curve |
| Utility Simulator | 0xe9ac43e7 | Float32 | Power line scaling factor | Array of scaling factors for the width of the power lines, one per zoom level. |
| Utility Simulator | 0x0911e2e8 | Float32 | Funding percentage to decay rate multiplier response curve | Multiples the rate of aging based on funding level |
| Utility Simulator | 0x0911e2e9 | Float32 | Usage percentage to decay rate multiplier response curve | Multiples the rate of aging based on usage level |
| Utility Simulator | 0xaa1f8ab7 | Float32 | Garbage service cost per unit of garbage | Cost per unit of garbage sent to landfill, incinerator or waste-to-energy |
| Utility Simulator | 0x8a235c56 | Float32 | Pipe burst probability | Likelihood per month that a distressed pipe will burst |
| Utility Simulator | 0x0a2eced8 | Float32 | Power Pole Demolition Cost | Cost to demolish a power pole. Lines are ignored |
| Utility Simulator | 0x6a406201 | Uint32 | Air Pollution Delta | Maximum amount of air pollution that can be added to/subtracted from each cell per month. |
| Utility Simulator | 0x8a40620b | Uint32 | Water Pollution Delta | Maximum amount of water pollution that can be added to/subtracted from each cell per month. |
| Utility Simulator | 0xea4025a9 | Float32 | Water Department Max Funding | Max funding percentage for the water department |
| Utility Simulator | 0x6a410bf2 | Float32 | Water Funding To Efficiency Curve | Maps difference between actual % funding and % ideal funding to % of a utility output |
| Utility Simulator | 0x4a652679 | Uint32 | Max Garbage on Valve | Maximum amount of garbage units the valve can hold |
| Utility Simulator | 0x4a65267a | Uint32 | Landfill Tile Capacity | Amount of garbage units each landfill tile can hold |
| Utility Simulator | 0x8c19c3e9 | Float32 | Garbage Transport Versus Landfill Tiles | How much garbage can be delivered to the landfill each month |
| Utility Simulator | 0xaa713782 | Float32 | Funding percentage to power pole decay/repair | Converts a funding level (percentage) into a decay rate (negative numbers) or repair rate (positive numbers) |
| Utility Simulator | 0xca7138c4 | Float32 | Funding percentage to pipe decay/repair | Converts a funding level (percentage) into a decay rate (negative numbers) or repair rate (positive numbers) |
| Utility Simulator | 0x8a84778f | Float32 | Monthly Cost Per Pipe | Maintenance cost for each pipe occupant |
| Traffic Simulator | 0x491332e6 | Float32 | Traffic Volume Per Population | Traffic volume generated per point of population in a building |
| Traffic Simulator | 0x491332e7 | Float32 | Max speed by Network for walking | Speed to walk through rd, rl, hwy, st, x, x, ave, sub, lr, mo, ow, dr, gh |
| Traffic Simulator | 0x491332e8 | Float32 | Max speed by Network for driving | Speed to drive through rd, rl, hwy, st, x, x, ave, sub, lr, mo, ow, dr, gh |
| Traffic Simulator | 0x491332e9 | Float32 | Max speed by Network for a bus | Speed to bus travel through rd, rl, hwy, st, x, x, ave, sub, lr, mo, ow, dr, gh |
| Traffic Simulator | 0x491332ea | Float32 | Max speed by Network for a train | Speed to train travel through rd, rl, hwy, st, x, x, ave, sub, lr, mo, ow, dr, gh |
| Traffic Simulator | 0x491332eb | Float32 | Max speed by Network for a truck | Speed to truck travel through rd, rl, hwy, st, x, x, ave, sub, lr, mo, ow, dr, gh |
| Traffic Simulator | 0x491332ec | Float32 | Max speed by Network for a freight train | Speed to train travel through rd, rl, hwy, st, x, x, ave, sub, lr, mo, ow, dr, gh |
| Traffic Simulator | 0x491332ed | Float32 | Max speed by Network for subways | Speed to subway travel. Set the eighth value. |
| Traffic Simulator | 0x491332ee | Float32 | Max speed by Network for el train | Speed to ride el train. Set the eighth value. |
| Traffic Simulator | 0x491332ef | Float32 | Max speed by Network for monorail | Speed to ride monorail. Set the ninth value. |
| Traffic Simulator | 0x29136788 | Float32 | Monthly Traffic Density Reduction | Traffic density multiplied by this each month |
| Traffic Simulator | 0x6a84493e | Float32 | Monthly cost for network tile | Upkeep cost per tile of each network type (rd, rl, hwy, st, x, x, ave, sub, lr) |
| Traffic Simulator | 0xa92356ae | Float32 | Max roads funding percent | Maximum allowed funding percentage for roads |
| Traffic Simulator | 0xa92356af | Float32 | Max mass transit funding percent | Maximum allowed funding percentage for mass transit |
| Traffic Simulator | 0xa92356b0 | Float32 | Damaged road extra step cost | Additional cost to travel on a damaged road tile |
| Traffic Simulator | 0xa92356b1 | Float32 | Income per tile by travel type | Income gained for each tile: walk, car, bus, rail, freight truck, freight train, subway, lt rl |
| Traffic Simulator | 0xa92356b2 | Uint8 | Mass Transit Usage Chance | Percent chance an RCI trip will allow use of mass transit |
| Traffic Simulator | 0xa92356b3 | Float32 | Network Traffic Capacity | Level at which traffic volume becomes uncomfortable to sims (rd, rl, hwy, st, x, x, ave, sub, lr) |
| Traffic Simulator | 0xa92356b4 | Bool | Travel type generates traffic | Order: walk, car, bus, train, subway, lt rl |
| Traffic Simulator | 0xa92356b5 | Bool | Travel type can reach destination | Order: walk, car, bus, train, subway, lt rl |
| Traffic Simulator | 0xa92356b8 | Uint32 | Maximum distance from origin to network | How far a trip can jump from its starting zone to a road/rail/etc |
| Traffic Simulator | 0xa92356b9 | Float32 | Congestion vs Speed | Maps speed multiplier as a function of congestion |
| Traffic Simulator | 0xa92356ba | Float32 | Commute trip max time | Maximum time for a commute trip in minutes |
| Traffic Simulator | 0xa92356bb | Float32 | Intersection and Turn Capacity Effect | Diminishing effect of an intersection or turn on capacity |
| Traffic Simulator | 0xa92356bc | Float32 | Trip Starting Cost by travel type | Starting overhead cost in time for each travel type |
| Traffic Simulator | 0xa92356bd | Float32 | Job Scaling Constant | Multiplies building capacity to determine number of jobs there |
| Traffic Simulator | 0xa92356be | Float32 | Population Background Traffic | Amount of background (non-trip) traffic volume generated per population point for R, C, and I zones |
| Traffic Simulator | 0xa92356bf | Bool | Travel type affected by traffic | Order: walk, car, bus, train, freight truck, freight train, subway, lt rl |
| Traffic Simulator | 0xea8c3cdb | Float32 | Trip Starting Cost by travel type for Mass Transit | Starting overhead cost in time for each travel type for mass transit preferred trips |
| Traffic Simulator | 0xcad64136 | Float32 | Trip Starting Cost by travel type for Car Pref | Starting overhead cost in time for each travel type for car preferred trips |
| Traffic Simulator | 0x4953e8a3 | Uint8 | Travel strategy percent WealthNone | Odds of selecting each travel strategy: Mass Transit, Car, Fastest |
| Traffic Simulator | 0x4953e8a4 | Uint8 | Travel strategy percent Wealth$ | Odds of selecting each travel strategy: Mass Transit, Car, Fastest |
| Traffic Simulator | 0x4953e8a5 | Uint8 | Travel strategy percent Wealth$$ | Odds of selecting each travel strategy: Mass Transit, Car, Fastest |
| Traffic Simulator | 0x4953e8a6 | Uint8 | Travel strategy percent Wealth$$$ | Odds of selecting each travel strategy: Mass Transit, Car, Fastest |
| Traffic Simulator | 0x49a2e8be | Float32 | Freight traffic scaling factor | Percentage of industrial bldg. population that will generate freight trips |
| Traffic Simulator | 0xca5f7821 | Float32 | Transit Switch Entry Cost vs. Budget | Maps the budget percent (0-100) to a multiplier on the time cost of using a transit switch |
| Traffic Simulator | 0x4a678060 | Float32 | Nearest Destination Attractiveness | Lower scores means more side explorations but also more CPU time (Pathfinding Heuristic) |
| Traffic Simulator | 0xca76013b | Float32 | Trip Length to Minutes Display Multiplier | Multiply the trip length map values (0-255, 255 = Max Commute Time) by this value to get a time in minutes for display purposes" |
| Traffic Simulator | 0xea7b5f06 | Float32 | Max Mass Transit Strategy Trip Length | Max time in raw trip length that the mass transit preferred strategy will go using mass transit |
| Traffic Simulator | 0x8c3caef5 | Float32 | Ferry Fare | Income per traveller from a ferry trip |
| Traffic Simulator | 0x4be09ec9 | Float32 | Congestion to Accident Probability | Maps congestion to accident probablity |
| Traffic Simulator | 0x4be09eca | Float32 | Capacity to Accident Probability | Maps capacity to accident probablity |
| Traffic Simulator | 0x4be09ecb | Float32 | Accident Duration | Time (in seconds) that a traffic accident lasts |
| Traffic Simulator | 0x4be09ecc | Float32 | Accident Check Period | Period (in seconds) that the traffic sim tests for accidents |
| Traffic Simulator | 0x0bd86fd4 | Float32 | Funding to Damage Acceleration Curve | Maps funding (as percentage) to road/rail damage acceleration |
| Traffic Simulator | 0x0bd86fd5 | Float32 | Rail Damage Accident Factor | Probability (0 to 1.0) that a train going over a rail pothole will cause derailment |
| Residential Simulator | 0x2918949e | Float32 | Health Funding To Efficiency Curve | Maps difference between actual % funding and % ideal funding to % of Health output |
| Residential Simulator | 0x691894a4 | Float32 | Health Strike Response Curve | Maps difference between actual % funding and % ideal funding to % chance of a Health strike |
| Residential Simulator | 0x0a53b2f9 | Float32 | Health Funding to Coverage Curve | Maps difference between actual % funding and % ideal funding to % coverage |
| Residential Simulator | 0xe918946f | Float32 | (OBS) Health Min Acceptable Funding % | Minimum acceptable percentage for Health department funding |
| Residential Simulator | 0xa9189479 | Float32 | Health Init/Max Allowed Funding | Initial and Maximum allowed funding percentage for health department |
| Residential Simulator | 0x89189482 | Float32 | Health Strike Efficiency % | Efficiency of Health buildings will drop to this % during a strike |
| Residential Simulator | 0x8918948b | Uint32 | Health Strike Duration | Maximum duration of a Health strike, in months |
| Residential Simulator | 0x89189495 | Float32 | Health Min Grade to End Strike | Minimum grade that all health buildings must have to end a Health strike |
| Residential Simulator | 0x691894c6 | Float32 | School Funding To Efficiency Curve | Maps difference between actual % funding and % ideal funding to % of Education output |
| Residential Simulator | 0xc91894cb | Float32 | School Strike Response Curve | Maps difference between actual % funding and % ideal funding to % chance of a School strike |
| Residential Simulator | 0xea53b30f | Float32 | School Funding to Coverage Curve | Maps difference between actual % funding and % ideal funding to % coverage |
| Residential Simulator | 0xc91894a8 | Float32 | (OBS) School Min Acceptable Funding % | Minimum acceptable percentage for School department funding |
| Residential Simulator | 0x891894ad | Float32 | School Init/Max Allowed Funding | Initial and Maximum allowed funding percentage for schools |
| Residential Simulator | 0xa91894bc | Uint32 | School Strike Duration | Maximum duration of a School strike, in months |
| Residential Simulator | 0xe91894b3 | Float32 | School Strike Efficiency % | Efficiency of School buildings will drop to this % during a strike |
| Residential Simulator | 0x691894c0 | Float32 | School Min Grade to End Strike | Minimum grade that all schools must have to end a School strike |
| Residential Simulator | 0xc929c433 | Float32 | EQ Decay | Amount by which EQ decays each month |
| Residential Simulator | 0x0929c439 | Float32 | HQ Decay | Amount by which HQ decays each month |
| Residential Simulator | 0xc929c652 | Uint8 | Low Wealth Average Age | Average age assigned to new Low Wealth residents |
| Residential Simulator | 0x0929cc30 | Uint8 | Medium Wealth Average Age | Average age assigned to new Medium Wealth residents |
| Residential Simulator | 0x0929cc36 | Uint8 | High Wealth Average Age | Average age assigned to new High Wealth residents |
| Residential Simulator | 0xc929cc58 | Uint8 | Low Wealth EQ | EQ assigned to new Low Wealth residents |
| Residential Simulator | 0x0929cc65 | Uint8 | Medium Wealth EQ | EQ assigned to new Medium Wealth residents |
| Residential Simulator | 0x2929cc6b | Uint8 | High Wealth EQ | EQ assigned to new High Wealth residents |
| Residential Simulator | 0xc929cc8d | Uint8 | Low Wealth HQ | HQ assigned to new Low Wealth residents |
| Residential Simulator | 0x6929cc92 | Uint8 | Medium Wealth HQ | HQ assigned to new Medium Wealth residents |
| Residential Simulator | 0x4929cc97 | Uint8 | High Wealth HQ | HQ assigned to new High Wealth residents |
| Residential Simulator | 0x6941a67f | Float32 | Health Quotient to Life Expectancy Curve | Maps HQ of a tract to the Life Expectancy of that tract |
| Residential Simulator | 0x4941a92b | Float32 | Life Expectancy to Workforce % Curve | Maps Life Expectancy of a tract to the % of that tract that is in the workforce |
| Residential Simulator | 0x6941b022 | Float32 | Air Pollution to HQ Decay Curve | Maps amount of air pollution in a tract to the rate at which the HQ in that tract decays |
| Residential Simulator | 0xa941b028 | Float32 | Water Pollution to HQ Decay Curve | Maps amount of water pollution in a tract to the rate at which the HQ in that tract decays |
| Residential Simulator | 0xc941b02e | Float32 | Garbage Pollution to HQ Decay Curve | Maps amount of garbage pollution in a tract to the rate at which the HQ in that tract decays |
| Residential Simulator | 0xca027307 | Uint8 | Average Age reset value | When average age reaches life expectancy in a tract, the tract's average age is set to this value" |
| Residential Simulator | 0xca027325 | Float32 | EQ reset percent | When average age reaches life expectancy in a tract, the tract's EQ is multiplied by this value |
| Residential Simulator | 0xca027328 | Float32 | HQ reset percent | When average age reaches life expectancy in a tract, the tract's HQ is multiplied by this value |
| Residential Simulator | 0xea540621 | Float32 | Avg. Age to 1-10 Age Group | How average age translates to the distribution of population into the 1-10 age group |
| Residential Simulator | 0xea540622 | Float32 | Avg. Age to 11-20 Age Group | How average age translates to the distribution of population into the 11-20 age group |
| Residential Simulator | 0xea540623 | Float32 | Avg. Age to 21-30 Age Group | How average age translates to the distribution of population into the 21-30 age group |
| Residential Simulator | 0xea540624 | Float32 | Avg. Age to 31-40 Age Group | How average age translates to the distribution of population into the 31-40 age group |
| Residential Simulator | 0xea540625 | Float32 | Avg. Age to 41-50 Age Group | How average age translates to the distribution of population into the 41-50 age group |
| Residential Simulator | 0xea540626 | Float32 | Avg. Age to 51-60 Age Group | How average age translates to the distribution of population into the 51-60 age group |
| Residential Simulator | 0xea540627 | Float32 | Avg. Age to 61-70 Age Group | How average age translates to the distribution of population into the 61-70 age group |
| Residential Simulator | 0xea540628 | Float32 | Avg. Age to 71-80 Age Group | How average age translates to the distribution of population into the 71-80 age group |
| Residential Simulator | 0xea540629 | Float32 | Avg. Age to 81-90 Age Group | How average age translates to the distribution of population into the 81-90 age group |
| Residential Simulator | 0x8a891421 | Uint32 | School Capacity To Staff Divisor | The school's capacity is divided by this number to determine the number of teachers on staff |
| Residential Simulator | 0x8a89142a | Uint32 | Hospital Capacity To Staff Divisor | The hospital's capacity is divided by this number to determine the number of doctors on staff |
| Residential Simulator | 0x6a9f6edd | Float32 | Coverage radius slop multiplier | Coverage radii are multiplied by this value when computing tract coverage |
| Residential Simulator | 0x4aa84161 | Float32 | Maximum EQ Boost | Max EQ boost achievable without reward buildings. Used to scale values for Sims opinion polls. |
| Residential Simulator | 0x2aa84164 | Float32 | Maximum HQ Boost | Max HQ boost achievable without reward buildings. Used to scale values for Sims opinion polls. |
| Residential Simulator | 0x6ab70da7 | Float32 | Death's door HQ | If a tract's HQ falls below this number, they're at death's door." |
| Residential Simulator | 0x4ab70dac | Float32 | Death's door effect chance | Percent chance (0-100.0) that the effect will play for a tract that's at death's door. |
| Residential Simulator | 0x6ab71654 | Float32 | Death's door effect height | Base height above terrain at which the effect will play for a tract that's at death's door. |
| Residential Simulator | 0x4ab71659 | Float32 | Death's door effect height delta | Random height variance for effect for a tract that's at death's door. |
| Residential Simulator | 0xec43826b | Uint32 | School Capacity Chunk Size | How many people are represented by one unit in the coverage map |
| Residential Simulator | 0xac43826d | Uint32 | Hospital Capacity Chunk Size | How many people are represented by one unit in the coverage map |
| Police/Jail Simulator | 0x48d71ed0 | Sint32 | Police Station, Center Strength | (in Protection units) |
| Police/Jail Simulator | 0x48d71ed1 | Sint32 | Police Station, Exterior Strength | (in Protection units) |
| Police/Jail Simulator | 0x48d71ed2 | Float32 | Police Station, Radius | (in meters) |
| Police/Jail Simulator | 0x48d71ed5 | Sint32 | Police Station, Total Dispatches | Number of trucks this station can send |
| Police/Jail Simulator | 0x48d71ed9 | Sint32 | Police Station, Dispatch Center Strength | (in Protection units) |
| Police/Jail Simulator | 0x48d71eda | Sint32 | Police Station, Dispatch Exterior Strength | (in Protection units) |
| Police/Jail Simulator | 0x48d71edb | Float32 | Police Station, Dispatch Radius | (in meters) |
| Police/Jail Simulator | 0xcc0b0705 | Sint32 | Police Station, Total Copter Dispatches | Number of police helicopters this station can send |
| Police/Jail Simulator | 0x48d71ee0 | Sint32 | Jail, Inmate Capacity | Jail Inmate Capacity |
| Police/Jail Simulator | 0x48d71ee4 | Sint32 | Jail, Overcapacity Cost (per person) | Monthly cost, per convict, for every prisoner over regular (100%) capacity |
| Police/Jail Simulator | 0x68ddae92 | Float32 | Police, Max Jail Overcrowding Percentage | 0.0-100.0, See ISC4PoliceSimulator.h |
| Police/Jail Simulator | 0x68ddae93 | Float32 | Police, Funding To Efficiency Response Curve | See ISC4PoliceSimulator.h |
| Police/Jail Simulator | 0x68ddae95 | Float32 | Police, Max Allowed Funding Percentage | 0.0-100.0, See ISC4PoliceSimulator.h |
| Police/Jail Simulator | 0x68ddae97 | Float32 | Police, Monthly Criminal Rehabilitation Percent | 0.0-100.0, See ISC4PoliceSimulator.h |
| Police/Jail Simulator | 0x68ddae9a | Float32 | Police, Strike Efficiency | 0.0-100.0, See ISC4PoliceSimulator.h |
| Police/Jail Simulator | 0x68ddae9b | Uint32 | Police, Max Strike Duration | In days. See ISC4PoliceSimulator.h |
| Police/Jail Simulator | 0x68ddae9c | Float32 | Police, Min Funding To End Strikes | 0.0-100.0, See ISC4PoliceSimulator.h |
| Police/Jail Simulator | 0x68ddae9e | Float32 | Police, Strike Response Curve | See ISC4PoliceSimulator.h |
| Police/Jail Simulator | 0x68ddae9f | Uint32 | Police, Arrest Threshold | If crime is committed in a cell whose police protection value is above this, they'll get arrested |
| Police/Jail Simulator | 0xea614f63 | Float32 | Jail, Jailbreak Response Curve | Maps REAL jail capacity to jailbreak chance |
| Police/Jail Simulator | 0xea614f64 | Float32 | Jail, Jailbreak Escapee Percentage | Min/max percentage (0.0-100.0) of jail capacity that will escape during jailbreaks |
| Police/Jail Simulator | 0xea614f65 | Float32 | Jail, End Jailbreak Capacity | Stop any jailbreak in progress when REAL jail capacity percentage goes below this value |
| Police/Jail Simulator | 0x68ddaea0 | Float32 | Jail, Funding To Efficiency Response Curve | See ISC4PoliceSimulator.h |
| Police/Jail Simulator | 0x68ddae91 | Uint32 | (Obs) Police, Update Period | In days. See ISC4PoliceSimulator.h |
| Police/Jail Simulator | 0x68ddae94 | Float32 | (Obs) Police, Min Acceptable Funding Percentage | 0.0-100.0, See ISC4PoliceSimulator.h |
| Police/Jail Simulator | 0x68ddae96 | Uint32 | (Obs) Police, Max Acceptable Crime Level | See ISC4PoliceSimulator.h |
| Police/Jail Simulator | 0x68ddae98 | Float32 | (Obs) Police, Conviction Percentage | 0.0-100.0, See ISC4PoliceSimulator.h |
| Police/Jail Simulator | 0x68ddae99 | Float32 | (Obs) Police, Unfailed Criminal Efficiency Percentage | 0.0-100.0, See ISC4PoliceSimulator.h |
| Police/Jail Simulator | 0x68ddae9d | Float32 | (Obs) Police, Property Crimes Value Ratio | See ISC4PoliceSimulator.h |
| Police/Jail Simulator | 0x48d71ed3 | Sint32 | (Obs) Police Station, Monthly Upkeep Cost | (in Simoleons) |
| Police/Jail Simulator | 0x48d71ee2 | Sint32 | (Obs) Jail, Monthly Upkeep Cost | Jail Monthly Upkeep Cost |
| Crime Simulator | 0xaa12fcb0 | String | Crime Name |  |
| Crime Simulator | 0xaa12fcb1 | Uint32 | Icon Resource Key | TGI |
| Crime Simulator | 0xaa12fcb2 | Uint8 | Zone Filter | Zone types compatible with crime (none == can appear in any zone) |
| Crime Simulator | 0xaa12fcb3 | Uint8 | Wealth Filter | Zone wealth values compatible with crime (none == can appear around any wealth) |
| Crime Simulator | 0xaa12fcb4 | Uint8 | Zone Purpose Filter | Zone purpose types compatible with crime (none == can appear around any purpose) |
| Crime Simulator | 0xaa12fcb5 | Uint8 | Crime Grid Value | The crime grid tract value will be increased by this amount |
| Crime Simulator | 0xaa12fcb6 | Uint8 | Trigger Fire | If set to non-zero value, this will trigger a fire |
| Crime Simulator | 0xaa12fcb7 | Uint8 | Lot Condition Filter | Lot conditions compatible with crime |
| Crime Simulator | 0xaa12fcb8 | Float32 | Relative Occurrance | Relative occurance weight of this crime, only has meaning relative to values for other crimes. |
| Crime Simulator | 0xe8ec2720 | Uint32 | Crime Update Period | How often the Crime simulator updates, in days |
| Crime Simulator | 0xe8ec2721 | Float32 | Crime Police Factor | Crime Police Factor |
| Crime Simulator | 0x49a34301 | Float32 | Crime Prop Request Curve | Crime level (0-1) vs. Prop Share |
| Crime Simulator | 0x8a0cbaa3 | Float32 | Crime, Jobless Indicator Smoothing factor |  |
| Crime Simulator | 0x8a0cbaa4 | Float32 | Crime, R$: EQ -> Base Criminality Curve |  |
| Crime Simulator | 0x8a0cbaa5 | Float32 | Crime, R$$: EQ -> Base Criminality Curve |  |
| Crime Simulator | 0x8a0cbaa6 | Float32 | Crime, R$$$: EQ -> Base Criminality Curve |  |
| Crime Simulator | 0x8a0cbaa7 | Float32 | Crime, R$: Jobless ratio -> Criminality Curve |  |
| Crime Simulator | 0x8a0cbaa8 | Float32 | Crime, R$$: Jobless ratio -> Criminality Curve |  |
| Crime Simulator | 0x8a0cbaa9 | Float32 | Crime, R$$$: Jobless ratio -> Criminality Curve |  |
| Crime Simulator | 0x8a0cbaaa | Float32 | Crime, Criminality -> Crimes Curve |  |
| Crime Simulator | 0x8a0cbaab | Float32 | Crime, Police Protection -> Crime Attenuation |  |
| Crime Simulator | 0x8a0cbaac | Float32 | Crime, Crime Grid Decay Factor | Example: 0.5 means values get halved every month |
| Crime Simulator | 0x8a0cbaad | Float32 | Crime, Criminal Grid Decay Factor | Fraction (0 to 1.0) of unjailed criminals that will commit crimes next month |
| Fire Protection Simulator | 0x08f55dc0 | Sint32 | Fire Station, Center Strength | (in Fire Protection units) |
| Fire Protection Simulator | 0x08f55dc1 | Sint32 | Fire Station, Exterior Strength | (in Fire Protection units) |
| Fire Protection Simulator | 0x08f55dc2 | Float32 | Fire Station, Radius | (in meters) |
| Fire Protection Simulator | 0x08f55dc5 | Sint32 | Fire Station, Total Dispatches | Number of trucks this station can send |
| Fire Protection Simulator | 0x29233e9a | Sint32 | Fire Station, Dispatch Center Strength | (in Fire Protection units) |
| Fire Protection Simulator | 0x29233e9b | Sint32 | Fire Station, Dispatch Exterior Strength | (in Fire Protection units) |
| Fire Protection Simulator | 0x29233e9c | Float32 | Fire Station, Dispatch Radius | (in meters) |
| Fire Protection Simulator | 0x0c0afdd7 | Sint32 | Fire Station, Total Plane Dispatches | Number of planes this station can send |
| Fire Protection Simulator | 0x29233e8d | Float32 | FireProtectionSim, Funding Efficiency Curve | See ISC4FireProtectionSimulator.h |
| Fire Protection Simulator | 0x29233e8f | Float32 | FireProtectionSim, Max Allowed Funding | 100.0+ Maximum percentage you can fund a fire station |
| Fire Protection Simulator | 0x29233e94 | Float32 | FireProtectionSim, Strike Efficiency Multiplier | 0.0-1.0 efficiency multiplier when stations are on strike |
| Fire Protection Simulator | 0x29233e95 | Sint32 | FireProtectionSim, Max Strike Duration | Maximum number of days a strike can last |
| Fire Protection Simulator | 0x29233e96 | Float32 | FireProtectionSim, Min Funding To End Strikes | 0.0-100.0, See ISC4FireProtectionSimulator.h |
| Fire Protection Simulator | 0x29233e98 | Float32 | FireProtectionSim, Strike Response Curve | See ISC4FireProtectionSimulator.h" |
| Fire Protection Simulator | 0x69b425db | Float32 | FireProtectionSim, StationNoRoad Multiplier | (0.0 - 1.0) |
| Fire Protection Simulator | 0x08f55dc3 | Sint64 | (Obs) Fire Station, Monthly Upkeep Cost | in Simoleans |
| Fire Protection Simulator | 0x08f55dc4 | Float32 | (Obs) Fire Station, Funding Percentage | Percentage of funding |
| Flamability Simulator | 0xc9b40128 | Float32 | Flammability multiplier, summer | Flammability multiplier during summer |
| Flamability Simulator | 0x29b40134 | Float32 | Flammability multiplier, abandonment | Flammability multiplier if building if abandoned |
| Flamability Simulator | 0x29b4013b | Float32 | Flammability multiplier, no water | Flammability multiplier if building is not watered |
| Flamability Simulator | 0x29b4013c | Uint32 | Flammability, hotspot settings | Hotspot settings: threshold value (0-255), radius (in tracts), minimum weight |
| Flamability Simulator | 0x29b4013d | Float32 | Flammability, hotspot fire probability | Maps hotspot weight (# of cells above threshold) to monthly fire probability (in percent 0-100) |
| Flamability Simulator | 0x29b4013e | Float32 | Flammability, random fire probability | Monthly fire probability (in percent 0-100) of random lot fire |
| Aura Simulator | 0xea023270 | Float32 | Aura, Park Effect | Maps grid data to aura effect |
| Aura Simulator | 0xea023271 | Float32 | Aura, Pollution Effect, Air | Maps grid data to aura effect |
| Aura Simulator | 0xea023272 | Float32 | Aura, Pollution Effect, Water | Maps grid data to aura effect |
| Aura Simulator | 0xea023273 | Float32 | Aura, Pollution Effect, Garbage | Maps grid data to aura effect |
| Aura Simulator | 0xea023274 | Float32 | Aura, Pollution Effect, Radiation | Maps grid data to aura effect |
| Aura Simulator | 0xea023275 | Float32 | Aura, School Effect | Maps grid data to aura effect |
| Aura Simulator | 0xea023276 | Float32 | Aura, Hospital Effect | Maps grid data to aura effect |
| Aura Simulator | 0xea023277 | Float32 | Aura, Crime Effect | Maps grid data to aura effect |
| Aura Simulator | 0xea023278 | Float32 | Aura, Trip Length | Maps grid data to aura effect |
| Aura Simulator | 0xea023279 | Float32 | Aura, Traffic | Maps grid data to aura effect |
| Aura Simulator | 0xea02327a | Float32 | Aura, Long Term Decay Factor | 1.0 means no decay, 0.0 means full decay (no smoothing) |
| Aura Simulator | 0xea02327b | Float32 | Aura, Short Term Decay Factor | 1.0 means no decay, 0.0 means full decay (no smoothing) |
| Aura Simulator | 0xea02327c | Sint32 | Aura, Base Aura (Low) | Base aura for R$,Cs$,Id occupancy |
| Aura Simulator | 0xea02327d | Sint32 | Aura, Base Aura (Med) | Base aura for R$$,Cs$$,Co$$,Ir,Im occupancy |
| Aura Simulator | 0xea02327e | Sint32 | Aura, Base Aura (High) | Base aura for R$$$,Cs$$$,Co$$$ occupancy |
| Aura Simulator | 0xea02327f | Float32 | Aura, Tax Rate Change Effect | Maps tax rate change (in percent) to aura effect |
| Aura Simulator | 0xea023280 | Float32 | Aura, Value/Wealth Effect | Maps (value_type-wealth_type) to aura effect |
| Aura Simulator | 0xea023290 | Float32 | Aura, Transient Effect, Fire | Magnitude and radius in meters of transient aura effect |
| Aura Simulator | 0xea023291 | Float32 | Aura, Transient Effect, Flora | Magnitude and radius in meters of transient aura effect |
| Aura Simulator | 0xea023292 | Float32 | Aura, Transient Effect, Civic Plop | Magnitude and radius in meters of transient aura effect |
| Aura Simulator | 0xea023293 | Float32 | Aura, Transient Effect, Network Plop | Magnitude and radius in meters of transient aura effect |
| Aura Simulator | 0xea0232a0 | Sint32 | Aura, Hotspot Settings | Hotspot settings: threshold value (0-255), radius (in tracts), minimum weight |
| Aura Simulator | 0xea0232a1 | Float32 | Aura, Hotspot Riot Probability | Maps hotspot weight (# of cells above threshold) to monthly fire probability (in percent 0-100) |
| Other | 0x47e2c200 | Uint8 | Tract Size | Power of 2; determines the size of the sim grid |
| Other | 0x68f26014 | Uint32 | Map View Unsigned Colors | RGBA color of 3D overlay of 2-color map view |
| Other | 0x68f26015 | Uint32 | Map View Signed Colors | RGBA color of 3D overlay of 3-color map view |
| Zone Manager | 0xe94825b2 | Sint32 | kZM_mZoneMinCellSize | kZM_mZoneMinCellSize |
| Zone Manager | 0xe94825b3 | Sint32 | kZM_mZoneMaxCellSize | kZM_mZoneMaxCellSize |
| Zone Manager | 0xe94825b4 | Sint64 | kZM_mZoneCreationCostArray | kZM_mZoneCreationCostArray |
| Zone Manager | 0xe94825b5 | Sint64 | kZM_mZoneDestructionCostArray | kZM_mZoneDestructionCostArray |
| Zone Manager | 0xe94825b6 | Uint32 | kZM_mZoneTextureResourceInstanceArray | kZM_mZoneTextureResourceInstanceArray |
| Zone Manager | 0xe94825b7 | Uint8 | Zone Display Alpha | Replaces drag color alpha after zone placement |
| Zone Manager | 0xe94825b8 | Uint32 | Zone Drag Color | RGBA color to blend with black and white texture NRRRCCCIIIMASSLP |
| Zone Manager | 0xe94825b9 | Float32 | Zone Max Slope | Meters per cell must be less than this value NRRRCCCIIIMASSLP |

## RCI demand / growth developer

| Section | Property ID | Type | Name | Description |
|---|---|---|---|---|
| RCI | 0x47bb3f10 | Sint32 | Minimum Value | Minimum active demand, use 0x80000000 for none |
| RCI | 0x47bb3f11 | Sint32 | Maximum Value | Maximum active demand, use 0x7fffffff for none |
| RCI | 0x47bb3f20 | Sint32 | Initial Supply | Initial supply of this resource, [min, max] |
| RCI | 0x47bb3f21 | Sint32 | Initial Demand | Initial demand for this resource, [min, max] |
| RCI | 0x47bb3f30 | Uint32 | Drives | Demands increased by this resource (demandID, percentage) |
| RCI | 0x47bb3f31 | Uint32 | Satisfies | Resources created by this resource (demandID, percentage) |
| RCI | 0x47bb3f32 | Uint32 | Contributes | Demands increased by this demand (demandID, percentage) |
| RCI | 0x47bb3f41 | Uint32 | Caps demand | Demand IDs capped by this resource, based on % depleted |
| RCI | 0x47bb3f50 | Uint32 | Sums | List of demand IDs summed by this one |
| RCI | 0x47bb3f60 | Uint8 | Region Use | Which part of the regional index is added to the local index |
| RCI | 0x47bb3f40 | Uint32 | Limits growth | Supply limited by this resource (demandID, percentage) |
| Landfill Developer | 0x4a280510 | Float32 | Landfill: Garbage Decay Rate | The rate at which garbage decays in units per month |
| Landfill Developer | 0x4a280513 | Float32 | Landfill: Level Tolerance Threshold | The altitude variance of the landfill zone before levelling will take place |
| Landfill Developer | 0x4a677e5b | Uint32 | Landfill: Total Max Garbage Monthly | The max total amount of garbage units taken in by landfills monthly |
| Landfill Developer | 0x4a677e5c | Uint32 | Landfill: Monthly Max Garbage per Landfill | The max amount of garbage units taken in by each landfill monthly |
| Landfill Developer | 0x4a677e5d | Bool | Landfill: Move Landfill Tiles with Terrain | Flag indicating whether or not to move landfill tiles with terrain |
| Landfill Developer | 0x4a677e5e | Uint32 | Landfill: Monthly Max Garbage Decayed per Landfill | The max amount of garbage units to decay in each landfill monthly |
| Landfill Developer | 0x4a677e5f | Sint64 | Landfill: Default Landfill Cell Maintenance Cost | Monthly maintenance cost of a landfill cell (can be overridden by the model budget cost property) |
| Landfill Developer | 0x2aa00fe7 | Sint64 | Landfill: Cost for Altitude Change | When zoning, the additional cost per cell for each meter of altitude changed |
| Port Developer | 0x8a270fc3 | Uint32 | Port Type Exemplar IDs | List of instance IDs for exemplars containing the properties for each distinct airport/seaport type |
| Port Developer | 0xca271780 | Uint32 | Num Growth Stages | Number of growth stages (i.e. number of lots) for this particular airport type |
| Port Developer | 0xca271781 | Uint32 | Lot stage template IDs | List of template IDs for each growth stage |
| Port Developer | 0xca271782 | Float32 | (Obs) Demand Thresholds List | List of demand thresholds for each growth stage. What this value refers to is zone type specific. |
| Port Developer | 0xca271783 | Sint64 | Budget Requirements List | List of budget requirements for each growth stage - in Simoleans (tm) |
| Port Developer | 0xca271784 | Uint32 | Trip Capacity List | List of trip capacities for each growth stage for airport or seaport |
| Port Developer | 0xca271785 | Float32 | Efficiency Distress Threshold | Efficiency percentage under which the port lot distresses |
| Port Developer | 0xca271786 | Sint64 | Stage Upgrade Cost | List of costs in Simoleans (tm) associated with each growth stage |
| Port Developer | 0xca271787 | Uint8 | Port Size | Port Size identifier. 1=small, 2=medium, 3=large |
| Port Developer | 0xca271788 | Uint8 | Port Width | Width of the port lot (in North rotation) |
| Port Developer | 0xca271789 | Uint8 | Port Height | Height of the port lot (in North rotation) |
| Port Developer | 0xca271800 | Float32 | Height Map | Optional list of vertex altitude values corresponding to each cell in the lot for airport or seaport |
| Park Manager | 0xca480830 | Float32 | Efficiency Distress Threshold | Efficiency value under which the lot begins to distress |
| Park Manager | 0x4a6676ad | Float32 | Zoo Escape Chance Curve | Input: park efficiency (0-100) Output: Chance of zoo escape each month (0-100) |
| General | 0x87cd6300 | Uint8 | Target Zone | The type of zone that is developed |
| General | 0x87cd6301 | Uint32 | Demand Source Index | Demand comes from this index OR source zone |
| General | 0x87cd6310 | Uint8 | Tract Size | Power of 2; determines the size of the demand grid |
| General | 0x87cd6330 | Uint8 | Wealth Type | Specifies (source and target) wealth type |
| General | 0x87cd6332 | Uint8 | Purpose Type | Specifies type of building created by this developer |
| General | 0x87cd6333 | Uint8 | Land Value Range | Specifies acceptable land values (0, 0 disables) |
| General | 0x87cd633f | Bool | Land Value Use Intrinsic | True means use total land value |
| General | 0x87cd6340 | Float32 | Land Value Effect | How much land value is a factor in desirability |
| General | 0x87cd6341 | Float32 | Park Effect | How much park proximity affects desirability |
| General | 0x87cd6343 | Float32 | R$ Proximity Effect | How much proximity to R$ is a factor in desirability |
| General | 0x87cd6344 | Float32 | R$$ Proximity Effect | How much proximity to R$$ is a factor in desirability |
| General | 0x87cd6345 | Float32 | R$$$ Proximity Effect | How much proximity to R$$$ is a factor in desirability |
| General | 0x87cd6346 | Float32 | Traffic Effect | How much traffic volume is a factor in desirability |
| General | 0x87cd6347 | Float32 | Trip Length Effect | How much travel time is a factor in desirability |
| General | 0x87cd6348 | Float32 | School Effect | How much school grade is a factor in desirability |
| General | 0x87cd6349 | Float32 | Hospital Effect | How much hospital grade is a factor in desirability |
| General | 0x87cd634a | Float32 | Crime Effect | How much crime is a factor in desirability |
| General | 0x87cd634b | Float32 | Pollution Effect Air | How much air pollution proximity affects desirability |
| General | 0x87cd634c | Float32 | Pollution Effect Water | How much water pollution proximity affects desirability |
| General | 0x87cd634d | Float32 | Pollution Effect Garbage | How much garbage pollution proximity affects desirability |
| General | 0x87cd634e | Float32 | Pollution Effect Radiation | How much radiation pollution proximity affects desirability |
| General | 0x87cd6354 | Sint32 | Desirability Threshold Decline | Tracts with desirability greater than this value never abandon. |
| General | 0x87cd6355 | Sint32 | Desirability Threshold Growth | Tracts with desirability less than this value never develop. Tested before pruning. |
| General | 0x87cd6356 | Bool | Phantom Demand | If true, unhappy sims will create a demand for new growth, which results in migration |
| General | 0x87cd6357 | Sint32 | Baseline Desirability | Value for desirability, if no other effects are applicable |
| General | 0x87cd6360 | Uint8 | Stage Count | Number of growth stages |
| General | 0x87cd6361 | Float32 | Stage 1 vs. Size | Percentage of growth which should be of type 1, as a function of the population of the city |
| General | 0x87cd6362 | Float32 | Stage 2 vs. Size | Percentage of growth which should be of type 2, as a function of the population of the city |
| General | 0x87cd6363 | Float32 | Stage 3 vs. Size | Percentage of growth which should be of type 3, as a function of the population of the city |
| General | 0x87cd6364 | Float32 | Stage 4 vs. Size | Percentage of growth which should be of type 4, as a function of the population of the city |
| General | 0x87cd6365 | Float32 | Stage 5 vs. Size | Percentage of growth which should be of type 5, as a function of the population of the city |
| General | 0x87cd6366 | Float32 | Stage 6 vs. Size | Percentage of growth which should be of type 6, as a function of the population of the city |
| General | 0x87cd6367 | Float32 | Stage 7 vs. Size | Percentage of growth which should be of type 7, as a function of the population of the city |
| General | 0x87cd6368 | Float32 | Stage 8 vs. Size | Percentage of growth which should be of type 8, as a function of the population of the city |
| General | 0x87cd6370 | Uint8 | Water Supply Cap | Maximum density allowed without water service |
| General | 0x87cd6371 | Uint8 | Fire Coverage Cap | Maximum density allowed without fire protection |
| General | 0x87cd6372 | Uint8 | Park Cap | Maximum density allowed without a nearby park |
| General | 0x87cd6373 | Uint8 | Power Supply Cap | Maximum density allowed without power service |
| General | 0x87cd6381 | Uint8 | Use Monopoly Pieces | Specifies building selection mode |
| General | 0x87cd6391 | Float32 | Co$$ Proximity Effect | How much proximity to Co$$ is a factor in desirability |
| General | 0x87cd6392 | Float32 | Co$$$ Proximity Effect | How much proximity to Co$$$ is a factor in desirability |
| General | 0x87cd6397 | Float32 | Transient Aura Effect | How much transient aura effects (-128, 127) affect desirability |
| General | 0x87cd6398 | Float32 | Slope Effect | How much slope is a factor in desirability |
| General | 0x87cd6399 | Float32 | Landmark Effect | How much landmark proximity affects desirability |
| General | 0x87cd63a0 | Uint32 | Style Percentages | Pairs of occupant group IDs and percentages showing how often they should be used |
| General | 0x87cd6331 | Uint8 | (Obs) Density Type | Specifies density |
| General | 0x87cd6342 | Float32 | (Obs) Pollution Effect | How much (air, water, garbage, radiation) pollution proximity affects desirability |
| General | 0x87cd6380 | Float32 | (Obs) Abandonment Thresholds | Min occupancy rate [0,1]: low, medium, high density |

## Budget tuning

| Section | Property ID | Type | Name | Description |
|---|---|---|---|---|
| Budget Tuning | 0x6a9c7718 | Sint64 | Budget: Initial Funds | Starting funds for a new city, by difficult level |
| Budget Tuning | 0x890e0a99 | Float32 | Budget: MaxTaxRate | Maximum Tax Rate - something like maybe 25% |
| Budget Tuning | 0x090e0a9f | Sint64 | Budget: MinAllowableFunds | The minimum before you are bankrupt. Usually something like -100,000 |
| Budget Tuning | 0x490e0aa6 | Sint64 | Budget: BondIncrement | Bonds are issued in values that are of this increment |
| Budget Tuning | 0x090e0aac | Uint8 | Budget: MaxBonds | The maximum number of outstanding bonds that is allowed |
| Budget Tuning | 0x890e0ab4 | Sint64 | Budget: MaxBondAmount | The maximum amount of outstanding bond debt that is allowed |
| Budget Tuning | 0x8a3fb98d | Float32 | Budget: BondInterestRate | Interest applied to principal once and added to total |
| Budget Tuning | 0x0a3fb9a8 | Uint32 | Budget: BondPaybackMonths | Number of months to payback a bond |
| Budget Tuning | 0xa9188ef5 | Float32 | Budget: TaxRateOccWeight_R$ | Multiplier to tweak the relative weighting of the R$ occupant type's taxes |
| Budget Tuning | 0xe918c920 | Float32 | Budget: TaxRateOccWeight_R$$ | Multiplier to tweak the relative weighting of the R$$ occupant type's taxes |
| Budget Tuning | 0x4918c925 | Float32 | Budget: TaxRateOccWeight_R$$$ | Multiplier to tweak the relative weighting of the R$$$ occupant type's taxes |
| Budget Tuning | 0xa918c929 | Float32 | Budget: TaxRateOccWeight_Cs$ | Multiplier to tweak the relative weighting of the CS$ occupant type's taxes |
| Budget Tuning | 0x4918c92c | Float32 | Budget: TaxRateOccWeight_Cs$$ | Multiplier to tweak the relative weighting of the CS$$ occupant type's taxes |
| Budget Tuning | 0x4918c930 | Float32 | Budget: TaxRateOccWeight_Cs$$$ | Multiplier to tweak the relative weighting of the CS$$$ occupant type's taxes |
| Budget Tuning | 0x8918c937 | Float32 | Budget: TaxRateOccWeight_Co$$ | Multiplier to tweak the relative weighting of the CO$$ occupant type's taxes |
| Budget Tuning | 0x4918c93a | Float32 | Budget: TaxRateOccWeight_Co$$$ | Multiplier to tweak the relative weighting of the CO$$$ occupant type's taxes |
| Budget Tuning | 0x69a36e40 | Float32 | Budget: TaxRateOccWeight_IResource | Multiplier to tweak the relative weighting of the IResource occupant type's taxes |
| Budget Tuning | 0xa918c942 | Float32 | Budget: TaxRateOccWeight_IDirty | Multiplier to tweak the relative weighting of the IDirty occupant type's taxes |
| Budget Tuning | 0x8918c93e | Float32 | Budget: TaxRateOccWeight_IManufacturing | Multiplier to tweak the relative weighting of the IManufacturing occupant type's taxes |
| Budget Tuning | 0xa9a36e47 | Float32 | Budget: TaxRateOccWeight_IHighTech | Multiplier to tweak the relative weighting of the IHighTech occupant type's taxes |
| Budget Tuning | 0x8a480df7 | Float32 | Budget: Waning Wealth Property Tax Multiplier | Multiplier applied to a building tax value for each level of difference in wealth level |
| Budget Tuning | 0x4ac519b8 | Float32 | Budget: Land Value to Simoleans | For each taxable square of the given land value, the number of Simoleans produced |
| Budget Tuning | 0xebff69e7 | Float32 | Budget: Difficulty Tax Multiplier | Multiplier for tax revenue by difficulty type |
| Budget Tuning | 0x0c032488 | Float32 | Budget: Initial Tax Rate | Starting tax rate for all developer types, indexed by difficulty type |
