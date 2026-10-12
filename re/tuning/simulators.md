# SC4 simulators exemplars (group 0xE7E2C2DB)

Extracted read-only from C:\Program Files (x86)\GOG Galaxy\Games\SimCity 4 Deluxe Edition; later packages override earlier ones.


## 0x08ED34DD — UI  (SimCity_1.dat)

| Property | Name | Type | Value |
|---|---|---|---|
| 0x00000010 |  | u32 | 5 |
| 0x00000020 |  | string | UI |
| 0x68F26014 | Map View Unsigned Colors | u32 | 128, 4294967168 |
| 0x68F26015 | Map View Signed Colors | u32 | 128, 2155905152, 4294967168 |

## 0x0BFF6824 — Budget Tuning Parameters  (SimCity_1.dat)

| Property | Name | Type | Value |
|---|---|---|---|
| 0x00000010 |  | u32 | 5 |
| 0x00000020 |  | string | Budget Tuning Parameters |
| 0x090E0A9F | Budget: MinAllowableFunds | s64 | -100000 |
| 0x090E0AAC | Budget: MaxBonds | u8 | 10 |
| 0x0A3FB9A8 | Budget: BondPaybackMonths | u32 | 120 |
| 0x0C032488 | Budget: Initial Tax Rate | f32 | 9, 9, 9 |
| 0x490E0AA6 | Budget: BondIncrement | s64 | 5000 |
| 0x4918C925 | Budget: TaxRateOccWeight_R$$$ | f32 | 1 |
| 0x4918C92C | Budget: TaxRateOccWeight_Cs$$ | f32 | 1 |
| 0x4918C930 | Budget: TaxRateOccWeight_Cs$$$ | f32 | 1 |
| 0x4918C93A | Budget: TaxRateOccWeight_Co$$$ | f32 | 1 |
| 0x4998C89D |  | u8 | 0 |
| 0x4AC519B8 | Budget: Land Value to Simoleans | f32 | 0, 0, 255, 0 |
| 0x69A36E40 | Budget: TaxRateOccWeight_IResource | f32 | 1 |
| 0x6A9C7718 | Budget: Initial Funds | s64 | 500000, 200000, 100000 |
| 0x890E0A99 | Budget: MaxTaxRate | f32 | 25 |
| 0x890E0AB4 | Budget: MaxBondAmount | s64 | 200000 |
| 0x8918C937 | Budget: TaxRateOccWeight_Co$$ | f32 | 1 |
| 0x8918C93E | Budget: TaxRateOccWeight_IManufacturing | f32 | 1 |
| 0x8A3FB98D | Budget: BondInterestRate | f32 | 8.5 |
| 0x8A480DF7 | Budget: Waning Wealth Property Tax Multiplier | f32 | 1, 0.75, 0.5 |
| 0xA9188EF5 | Budget: TaxRateOccWeight_R$ | f32 | 1 |
| 0xA918C929 | Budget: TaxRateOccWeight_Cs$ | f32 | 1 |
| 0xA918C942 | Budget: TaxRateOccWeight_IDirty | f32 | 1 |
| 0xA9A36E47 | Budget: TaxRateOccWeight_IHighTech | f32 | 1 |
| 0xE918C920 | Budget: TaxRateOccWeight_R$$ | f32 | 1 |
| 0xEBFF69E7 | Budget: Difficulty Tax Multiplier | f32 | 1.2, 1.1, 1 |

## 0x28DFCF89 — Police Simulator  (SimCity_1.dat)

| Property | Name | Type | Value |
|---|---|---|---|
| 0x00000010 |  | u32 | 5 |
| 0x00000020 |  | string | Police Simulator |
| 0x68DDAE93 | Police, Funding To Efficiency Response Curve | f32 | 0, 0, 100, 100, 120, 110 |
| 0x68DDAE95 | Police, Max Allowed Funding Percentage | f32 | 120 |
| 0x68DDAE97 | Police, Monthly Criminal Rehabilitation Percent | f32 | 10 |
| 0x68DDAE9A | Police, Strike Efficiency | f32 | 20 |
| 0x68DDAE9B | Police, Max Strike Duration | s32 | 548 |
| 0x68DDAE9C | Police, Min Funding To End Strikes | f32 | 90 |
| 0x68DDAE9E | Police, Strike Response Curve | f32 | 0, 100, 25, 25, 50, 10, 70, 0 |
| 0x68DDAE9F | Police, Arrest Threshold | u32 | 20 |
| 0x68DDAEA0 | Jail, Funding To Efficiency Response Curve | f32 | 0, 0, 10, 50, 100, 100, 120, 110 |
| 0xEA614F63 | Jail, Jailbreak Response Curve | f32 | 0, 0, 80, 0, 90, 10, 100, 100 |
| 0xEA614F64 | Jail, Jailbreak Escapee Percentage | f32 | 10, 50 |
| 0xEA614F65 | Jail, End Jailbreak Capacity | f32 | 80 |

## 0x48EA97D4 — Ordinance Simulator  (SimCity_1.dat)

| Property | Name | Type | Value |
|---|---|---|---|
| 0x00000010 |  | u32 | 5 |
| 0x00000020 |  | string | Ordinance Simulator |
| 0xA8EAA060 |  | f32 | 5 |
| 0xA8EAA061 |  | u32 | 200 |
| 0xA8EAA062 |  | u32 | 30 |
| 0xA8EAA063 |  | u32 | 1960 |
| 0xA8EAA064 |  | u32 | 3 |
| 0xA8EAA065 |  | u32 | 3 |
| 0xA8EAA066 |  | u32 | 1 |
| 0xA8EAA067 |  | u32 | 1 |

## 0x4A1F3CE6 — Demand Simulator  (SimCity_1.dat)

| Property | Name | Type | Value |
|---|---|---|---|
| 0x00000010 |  | u32 | 5 |
| 0x00000020 |  | string | Demand Simulator |
| 0x4A1F38B5 | Demand: Neutral Tax Rate vs. Population | f32 | 0, 9, 150000, 8.9, 300000, 8.9, 450000, 8.8, 600000, 8.7, 750000, 8.7, 900000, 8.6, 1.05e+06, 8.5, 1.2e+06, 8.5, 1.35e+06, 8.4, 1.5e+06, 8.3, 1.65e+06, 8.3, 1.8e+06, 8.2, 1.95e+06, 8.1, 2.1e+06, 8, 2.25e+06, 8, 2.4e+06, 7.9, 2.55e+06, 7.8, 2.7e+06, 7.7, 2.85e+06, 7.6, 3e+06, 7.5, 3.15e+06, 7.4, 3.3e+06, 7.4, 3.45e+06, 7.3, 3.6e+06, 7.2, 3.75e+06, 7.1, 3.9e+06, 7, 4.05e+06, 6.9, 4.2e+06, 6.7, 4.35e … |
| 0x4A1F38B6 | Demand: Tax Modifier vs. Rate Variance | f32 | 0.2, 0.3, 1, 0, 5, -3.5 |
| 0x4A1F38B7 | Demand: Days of Tax Penalty | u32 | 90 |
| 0x4A1F38C5 | Demand: Commercical Cap Relief for Connections | f32 | 10000, 10000, 100000, 0, 0, 0, 20000, 20000, 20000, 20000, 0, 1000, 100000 |
| 0x4A1F38C6 | Demand: Commercical Cap Relief for Airports | f32 | 1 |
| 0x4A1F38C7 | Demand: Commercical Cap Relief for Seaports | f32 | 2 |
| 0x4A1F38C8 | Demand: Industrial Cap Relief for Trips | f32 | 20 |
| 0x4A1F38C9 | Demand: Industrial Cap Relief for Airports | f32 | 0.1 |
| 0x4A1F38CA | Demand: Industrial Cap Relief for Seaports | f32 | 20 |
| 0x8A63F2A1 | Average Salary for Demographic | u32 | 17000, 19250, 21500, 26000, 25000, 31250, 37500, 50000, 45000, 108750, 172500, 300000 |

## 0x4A1F6115 — Nation  (SimCity_1.dat)

| Property | Name | Type | Value |
|---|---|---|---|
| 0x00000010 |  | u32 | 5 |
| 0x00000020 |  | string | Nation |
| 0x4A1F629F | Nation: Demand ID list | u32 | 0 |
| 0x4A1F62A0 | Nation: Default Demand Modifier vs. Month | f32 | 0, 0 |
| 0x4A1F6300 | Nation: Difficulty Demand Modifier | f32 | 0.06, 0.03, 0.01 |

## 0x4AA4047A — ?  (SimCity_1.dat)

| Property | Name | Type | Value |
|---|---|---|---|
| 0x2AA405D3 |  | string | xa37dd45ffe100bfffcc9753aabac325f07cb3fa231144fe2e33ae4783feead2b8a73ff021fac326df0ef9753ab9cdf6573ddff0312fab0b0ff39779eaff312x |

## 0x68F5524A — Fire Protection Simulator  (SimCity_1.dat)

| Property | Name | Type | Value |
|---|---|---|---|
| 0x00000010 |  | u32 | 5 |
| 0x00000020 |  | string | Fire Protection Simulator |
| 0x29233E8D | FireProtectionSim, Funding Efficiency Curve | f32 | 0, 10, 50, 25, 75, 45, 90, 75, 100, 100, 120, 110 |
| 0x29233E8F | FireProtectionSim, Max Allowed Funding | f32 | 120 |
| 0x29233E94 | FireProtectionSim, Strike Efficiency Multiplier | f32 | 0.2 |
| 0x29233E95 | FireProtectionSim, Max Strike Duration | s32 | 548 |
| 0x29233E96 | FireProtectionSim, Min Funding To End Strikes | f32 | 90 |
| 0x29233E98 | FireProtectionSim, Strike Response Curve | f32 | 0, 100, 25, 25, 50, 10, 70, 0 |
| 0x69B425DB | FireProtectionSim, StationNoRoad Multiplier | f32 | 0.8 |

## 0x69189C84 — Residential simulator  (SimCity_1.dat)

| Property | Name | Type | Value |
|---|---|---|---|
| 0x00000010 |  | u32 | 5 |
| 0x00000020 |  | string | Residential simulator |
| 0x0929C439 | HQ Decay | f32 | 0.2 |
| 0x0929CC30 | Medium Wealth Average Age | u8 | 30 |
| 0x0929CC36 | High Wealth Average Age | u8 | 35 |
| 0x0929CC65 | Medium Wealth EQ | u8 | 40 |
| 0x0A53B2F9 | Health Funding to Coverage Curve | f32 | 0, 50, 100, 100 |
| 0x2918949E | Health Funding To Efficiency Curve | f32 | 0, 0, 100, 100, 120, 110 |
| 0x2929CC6B | High Wealth EQ | u8 | 60 |
| 0x2AA84164 | Maximum HQ Boost | f32 | 1.1 |
| 0x4929CC97 | High Wealth HQ | u8 | 60 |
| 0x4941A92B | Life Expectancy to Workforce % Curve | f32 | 50, 40, 100, 60 |
| 0x4AA84161 | Maximum EQ Boost | f32 | 1.3 |
| 0x4AB70DAC | Death's door effect chance | f32 | 1.5 |
| 0x4AB71659 | Death's door effect height delta | f32 | 10 |
| 0x691894A4 | Health Strike Response Curve | f32 | 0, 100, 80, 0 |
| 0x691894C0 | School Min Grade to End Strike | f32 | 90 |
| 0x691894C6 | School Funding To Efficiency Curve | f32 | 0, 0, 100, 100, 120, 110 |
| 0x6929CC92 | Medium Wealth HQ | u8 | 40 |
| 0x6941A67F | Health Quotient to Life Expectancy Curve | f32 | 0, 50, 20, 55, 40, 60, 60, 65, 200, 100 |
| 0x6941B022 | Air Pollution to HQ Decay Curve | f32 | 0, 0, 512, 0.1, 1024, 50 |
| 0x6A9F6EDD | Coverage radius slop multiplier | f32 | 1.1 |
| 0x6AB70DA7 | Death's door HQ | f32 | 2 |
| 0x6AB71654 | Death's door effect height | f32 | 50 |
| 0x89189482 | Health Strike Efficiency % | f32 | 20 |
| 0x8918948B | Health Strike Duration | u32 | 500 |
| 0x89189495 | Health Min Grade to End Strike | f32 | 90 |
| 0x891894AD | School Init/Max Allowed Funding | f32 | 25, 120 |
| 0x8A891421 | School Capacity To Staff Divisor | u32 | 30 |
| 0x8A89142A | Hospital Capacity To Staff Divisor | u32 | 30 |
| 0xA9189479 | Health Init/Max Allowed Funding | f32 | 25, 120 |
| 0xA91894BC | School Strike Duration | u32 | 500 |
| 0xA941B028 | Water Pollution to HQ Decay Curve | f32 | 0, 0, 512, 0.1, 1024, 50 |
| 0xAC43826D | Hospital Capacity Chunk Size | u32 | 500, 500, 2000 |
| 0xC91894CB | School Strike Response Curve | f32 | 0, 100, 80, 0 |
| 0xC929C433 | EQ Decay | f32 | 0.2 |
| 0xC929C652 | Low Wealth Average Age | u8 | 25 |
| 0xC929CC58 | Low Wealth EQ | u8 | 20 |
| 0xC929CC8D | Low Wealth HQ | u8 | 20 |
| 0xC941B02E | Garbage Pollution to HQ Decay Curve | f32 | 0, 0, 512, 0.2, 1024, 20 |
| 0xCA027307 | Average Age reset value | u8 | 40 |
| 0xCA027325 | EQ reset percent | f32 | 80 |
| 0xCA027328 | HQ reset percent | f32 | 80 |
| 0xE91894B3 | School Strike Efficiency % | f32 | 20 |
| 0xEA53B30F | School Funding to Coverage Curve | f32 | 0, 50, 100, 100, 120, 110 |
| 0xEA540621 | Avg. Age to 1-10 Age Group | f32 | 20, 24, 30, 6, 40, 9, 50, 4, 60, 0 |
| 0xEA540622 | Avg. Age to 11-20 Age Group | f32 | 20, 28, 30, 24, 40, 12, 50, 6, 60, 0 |
| 0xEA540623 | Avg. Age to 21-30 Age Group | f32 | 20, 31, 30, 27, 40, 12, 50, 6, 60, 0 |
| 0xEA540624 | Avg. Age to 31-40 Age Group | f32 | 20, 12, 30, 20, 40, 13, 50, 8, 60, 3 |
| 0xEA540625 | Avg. Age to 41-50 Age Group | f32 | 20, 5, 30, 12, 40, 23, 50, 19, 60, 13 |
| 0xEA540626 | Avg. Age to 51-60 Age Group | f32 | 20, 0, 30, 7, 40, 14, 50, 27, 60, 39 |
| 0xEA540627 | Avg. Age to 61-70 Age Group | f32 | 20, 0, 30, 4, 40, 11, 50, 20, 60, 31 |
| 0xEA540628 | Avg. Age to 71-80 Age Group | f32 | 20, 0, 30, 0, 40, 5, 50, 8, 60, 11 |
| 0xEA540629 | Avg. Age to 81-90 Age Group | f32 | 20, 0, 30, 0, 40, 1, 50, 2, 60, 3 |
| 0xEC43826B | School Capacity Chunk Size | u32 | 500, 750, 1000, 1000, 10000, 300, 1200 |

## 0xA8F41EE3 — Flammability Simulator  (SimCity_1.dat)

| Property | Name | Type | Value |
|---|---|---|---|
| 0x00000010 |  | u32 | 5 |
| 0x00000020 |  | string | Flammability Simulator |
| 0x29B40134 | Flammability multiplier, abandonment | f32 | 1.5 |
| 0x29B4013B | Flammability multiplier, no water | f32 | 1.25 |
| 0x29B4013C | Flammability, hotspot settings | u32 | 54, 8, 40 |
| 0x29B4013D | Flammability, hotspot fire probability | f32 | 0, 0, 39, 0, 40, 7, 50, 15, 70, 20, 100, 25 |
| 0x29B4013E | Flammability, random fire probability | f32 | 5 |
| 0xC9B40128 | Flammability multiplier, summer | f32 | 1 |

## 0xAA023079 — Aura Simulator  (SimCity_1.dat)

| Property | Name | Type | Value |
|---|---|---|---|
| 0x00000010 |  | u32 | 5 |
| 0x00000020 |  | string | Aura Simulator |
| 0xEA023270 | Aura, Park Effect | f32 | 0, 0, 50, 2, 100, 3 |
| 0xEA023271 | Aura, Pollution Effect, Air | f32 | 0, 10, 200, 7, 400, 0, 600, -10, 800, -20, 1024, -35 |
| 0xEA023272 | Aura, Pollution Effect, Water | f32 | 0, 10, 200, 7, 400, 0, 600, -10, 800, -20, 1024, -35 |
| 0xEA023273 | Aura, Pollution Effect, Garbage | f32 | 0, 10, 200, 7, 400, 3, 1800, -4, 2400, -16, 4096, -35 |
| 0xEA023274 | Aura, Pollution Effect, Radiation | f32 | 0, 0, 1, -50 |
| 0xEA023275 | Aura, School Effect | f32 | 0, -7, 1, 7 |
| 0xEA023276 | Aura, Hospital Effect | f32 | 0, -7, 1, 7 |
| 0xEA023277 | Aura, Crime Effect | f32 | 0, 10, 40, 5, 60, 2, 80, -3, 100, -5, 255, -50 |
| 0xEA023278 | Aura, Trip Length | f32 | 0, 7, 255, -7 |
| 0xEA023279 | Aura, Traffic | f32 | 0, 7, 255, -7 |
| 0xEA02327A | Aura, Long Term Decay Factor | f32 | 0.9 |
| 0xEA02327B | Aura, Short Term Decay Factor | f32 | 0.75 |
| 0xEA02327C | Aura, Base Aura (Low) | s32 | 0 |
| 0xEA02327D | Aura, Base Aura (Med) | s32 | 0 |
| 0xEA02327E | Aura, Base Aura (High) | s32 | 0 |
| 0xEA02327F | Aura, Tax Rate Change Effect | f32 | -20, 25, -10, 20, -5, 10, -2, 7, -1, 5, 1, -5, 2, -7, 5, -10, 10, -20, 20, -25 |
| 0xEA023280 | Aura, Value/Wealth Effect | f32 | -2, 5, -1, 3, 0, 0, 1, -3, 2, -5 |
| 0xEA023290 | Aura, Transient Effect, Fire | f32 | -20, 128 |
| 0xEA023291 | Aura, Transient Effect, Flora | f32 | 3, 32 |
| 0xEA023292 | Aura, Transient Effect, Civic Plop | f32 | 25, 128 |
| 0xEA023293 | Aura, Transient Effect, Network Plop | f32 | 0, 0 |
| 0xEA0232A0 | Aura, Hotspot Settings | s32 | -50, 3, 4 |
| 0xEA0232A1 | Aura, Hotspot Riot Probability | f32 | 0, 0, 5, 10, 10, 60, 20, 100 |

## 0xC8E8F6C5 — Crime Simulator  (SimCity_1.dat)

| Property | Name | Type | Value |
|---|---|---|---|
| 0x00000010 |  | u32 | 5 |
| 0x00000020 |  | string | Crime Simulator |
| 0x49A34301 | Crime Prop Request Curve | f32 | 0, 0, 0.1, 2, 0.2, 4, 0.4, 6, 1, 10 |
| 0x8A0CBAA4 | Crime, R$: EQ -> Base Criminality Curve | f32 | 0, 0.4, 100, 0.3, 200, 0 |
| 0x8A0CBAA5 | Crime, R$$: EQ -> Base Criminality Curve | f32 | 0, 0.15, 100, 0.11, 200, 0 |
| 0x8A0CBAA6 | Crime, R$$$: EQ -> Base Criminality Curve | f32 | 0, 0.1, 100, 0.07, 200, 0 |
| 0x8A0CBAA7 | Crime, R$: Jobless ratio -> Criminality Curve | f32 | -1, 0.1, -0.7, 0.07, 0, 0, 0.7, -0.07, 1, -0.1 |
| 0x8A0CBAA8 | Crime, R$$: Jobless ratio -> Criminality Curve | f32 | -1, 0.07, -0.7, 0.05, 0, 0, 0.7, -0.05, 1, -0.07 |
| 0x8A0CBAA9 | Crime, R$$$: Jobless ratio -> Criminality Curve | f32 | -1, 0.05, -0.7, 0.03, 0, 0, 0.7, -0.03, 1, -0.05 |
| 0x8A0CBAAA | Crime, Criminality -> Crimes Curve | f32 | 0, 0, 200, 1, 5000, 5, 10000, 10, 20000, 25, 200000, 40, 400000, 55, 600000, 60, 900000, 65 |
| 0x8A0CBAAB | Crime, Police Protection -> Crime Attenuation | f32 | 0, 0, 100, 0.05, 200, 0.2, 255, 0.5 |
| 0x8A0CBAAC | Crime, Crime Grid Decay Factor | f32 | 0.8 |
| 0x8A0CBAAD | Crime, Criminal Grid Decay Factor | f32 | 0.66 |

## 0xC911E35B — Utilities  (SimCity_1.dat)

| Property | Name | Type | Value |
|---|---|---|---|
| 0x00000010 |  | u32 | 5 |
| 0x00000020 |  | string | Utilities |
| 0x0911E2E6 | Utility Strike Response Curve | f32 | 0, 0, 20, 45, 50, 75, 100, 100 |
| 0x0911E2E8 | Funding percentage to decay rate multiplier response curve | f32 | 0, 1.1, 1, 1, 1.2, 0.9 |
| 0x0911E2E9 | Usage percentage to decay rate multiplier response curve | f32 | 0, 0.1, 0.25, 0.2, 0.5, 0.45, 0.75, 0.85, 0.9, 1.25, 1, 1.55, 1.1, 2 |
| 0x09122027 | Utility Strike Efficiency % | f32 | 20 |
| 0x09134BE2 | Max water pollution value for UI | u32 | 1024 |
| 0x0913519B | Garbage Pollution Threshold | u32 | 1500 |
| 0x091351AE | Water Pollution Threshold | u32 | 512 |
| 0x09135479 | Garbage decompostion amount | u32 | 2000 |
| 0x09135482 | Garbage UI scaling factor | f32 | 0.01 |
| 0x098B25C8 | Max cells between power poles | u32 | 10 |
| 0x098B27D0 | Minimum power line height | f32 | 7 |
| 0x0A10C772 | Water Pollution Effect Strength Curve | f32 | 0, 0, 0.002, 0.05, 0.005, 0.15, 0.01, 0.4, 0.02, 1 |
| 0x0A10C773 | Air Pollution Effect Strength Curve | f32 | 0, 0, 0.005, 0.05, 0.01, 0.25, 0.02, 0.5, 0.03, 0.6 |
| 0x0A10C774 | Radiation Pollution Effect Strength Curve | f32 | 0, 0, 0.001, 1, 0.01, 1 |
| 0x0A10C775 | Garbage Pollution Effect Strength Curve | f32 | 0, 0, 0.02, 0.02, 0.03, 0.06, 0.04, 0.2, 0.05, 0.4, 0.06, 0.53, 0.085, 0.75, 0.13, 1 |
| 0x0A2ECED8 | Power Pole Demolition Cost | f32 | 5 |
| 0x0A5F596B | Power line placement cost | f32 | 2 |
| 0x29121E8B | Power Min Acceptable Funding % | f32 | 60 |
| 0x2912202E | Utility Strike Duration | u32 | 12 |
| 0x291351B5 | Radiation Pollution Threshold | u32 | 1 |
| 0x49122036 | Utility Min Funding to End Strike | f32 | 90 |
| 0x49134BD4 | Max garbage value for UI | u32 | 32000 |
| 0x49134BDD | Max air pollution value for UI | u32 | 1024 |
| 0x49A195C0 | Power line curve distance between points | f32 | 8 |
| 0x49A1E8C4 | Power line control horiz. % | f32 | 10 |
| 0x4A5F5953 | Power pole placement cost | u32 | 15 |
| 0x4A652679 | Max Garbage on Valve | u32 | 65536 |
| 0x4A65267A | Landfill Tile Capacity | u32 | 30000 |
| 0x68F5436A | Water pipe effect radius | u32 | 7 |
| 0x68F5436B | Fresh water pump reach | u32 | 256 |
| 0x68F5436C | Salt water pump reach | u32 | 256 |
| 0x68F5436D | Fresh water distance loss | u32 | 100 |
| 0x6911E2DF | Power Funding To Efficiency Curve | f32 | 0, 0, 50, 25, 90, 80, 100, 100, 120, 110 |
| 0x69501944 | Traffic air pollution factor | f32 | 0.25 |
| 0x69A437F6 | Garbage Prop Request Curve | f32 | 0, 0, 0.0625, 5, 0.125, 10 |
| 0x6A406201 | Air Pollution Delta | u32 | 150 |
| 0x6A410BF2 | Water Funding To Efficiency Curve | f32 | 0, 0, 50, 30, 90, 80, 100, 100, 120, 110 |
| 0x89135044 | Max radiation pollution value for UI | u32 | 1024 |
| 0x89A18FC4 | Power line control max vertical distance | f32 | 10 |
| 0x89A1E8FB | Minimum number of curve points | u32 | 5 |
| 0x8A235C56 | Pipe burst probability | f32 | 0.03 |
| 0x8A40620B | Water Pollution Delta | u32 | 150 |
| 0x8A84778F | Monthly Cost Per Pipe | f32 | 0.1 |
| 0x8C19C3E9 | Garbage Transport Versus Landfill Tiles | f32 | 0, 0, 65536, 3.2768e+08 |
| 0xA911E2F1 | Power radiation radius | u32 | 2 |
| 0xA91351A6 | Air Pollution Threshold | u32 | 512 |
| 0xAA161F81 | Max acceptable pumped water pollution level | u32 | 512 |
| 0xAA1F8AB7 | Garbage service cost per unit of garbage | f32 | 0.005 |
| 0xAA713782 | Funding percentage to power pole decay/repair | f32 | 0, -0.02, 25, -0.005, 50, 0, 100, 0.025, 110, 0.05 |
| 0xCA7138C4 | Funding percentage to pipe decay/repair | f32 | 0, -0.01, 20, -0.005, 50, 0, 100, 0.02, 110, 0.05 |
| 0xE911E2D1 | Power max allowed funding | f32 | 120 |
| 0xE911E2FC | Power plant max production ratio | f32 | 110 |
| 0xE9A18D3F | Power line control max horiz. distance | f32 | 16 |
| 0xE9A1E8D4 | Power line control vertical % | f32 | 0.15 |
| 0xE9AC43E7 | Power line scaling factor | f32 | 0, 1, 0.5, 0.35, 0.2 |
| 0xEA4025A9 | Water Department Max Funding | f32 | 120 |

## 0xC9133286 — Traffic Simulator  (SimCity_1.dat)

| Property | Name | Type | Value |
|---|---|---|---|
| 0x00000010 |  | u32 | 5 |
| 0x00000020 |  | string | Traffic Simulator |
| 0x0BD86FD4 | Funding to Damage Acceleration Curve | f32 | 0, 0.02, 100, -0.01, 120, -0.04 |
| 0x0BD86FD5 | Rail Damage Accident Factor | f32 | 1 |
| 0x0CAAAEB8 |  | f32 | 0.128 |
| 0x29136788 | Monthly Traffic Density Reduction | f32 | 0 |
| 0x491332E6 | Traffic Volume Per Population | f32 | 2 |
| 0x491332E7 | Max speed by Network for walking | f32 | 3.5, 0, 0, 3.5, 0, 0, 3.5, 0, 0, 0, 3.5, 0, 0 |
| 0x491332E8 | Max speed by Network for driving | f32 | 31, 0, 82, 21, 0, 0, 40, 0, 0, 0, 31, 0, 82 |
| 0x491332E9 | Max speed by Network for a bus | f32 | 46, 0, 100, 31, 0, 0, 60, 0, 0, 0, 46, 0, 100 |
| 0x491332EA | Max speed by Network for a train | f32 | 0, 110, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 |
| 0x491332EB | Max speed by Network for a truck | f32 | 31, 0, 82, 21, 0, 0, 40, 0, 0, 0, 31, 0, 82 |
| 0x491332EC | Max speed by Network for a freight train | f32 | 0, 150, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 |
| 0x491332ED | Max speed by Network for subways | f32 | 0, 0, 0, 0, 0, 0, 0, 150, 0, 0, 0, 0, 0 |
| 0x491332EE | Max speed by Network for el train | f32 | 0, 0, 0, 0, 0, 0, 0, 0, 150, 0, 0, 0, 0 |
| 0x491332EF | Max speed by Network for monorail | f32 | 0, 0, 0, 0, 0, 0, 0, 0, 0, 200, 0, 0, 0 |
| 0x4953E8A3 | Travel strategy percent WealthNone | u8 | 0, 0, 100 |
| 0x4953E8A4 | Travel strategy percent Wealth$ | u8 | 80, 20, 0 |
| 0x4953E8A5 | Travel strategy percent Wealth$$ | u8 | 30, 0, 70 |
| 0x4953E8A6 | Travel strategy percent Wealth$$$ | u8 | 10, 80, 10 |
| 0x49A2E8BE | Freight traffic scaling factor | f32 | 0.05 |
| 0x4A678060 | Nearest Destination Attractiveness | f32 | 0.09 |
| 0x4BE09EC9 | Congestion to Accident Probability | f32 | 0, 0, 0.25, 0, 1, 0.05, 2, 0.25 |
| 0x4BE09ECA | Capacity to Accident Probability | f32 | 0, 1, 1000, 1 |
| 0x4BE09ECB | Accident Duration | f32 | 20 |
| 0x4BE09ECC | Accident Check Period | f32 | 60 |
| 0x6A84493E | Monthly cost for network tile | f32 | 0.1, 0.03, 0.5, 0.05, 0, 0, 0.15, 0.3, 0.2, 0.3, 0.1, 0, 0.5 |
| 0x8C3CAEF5 | Ferry Fare | f32 | 0.01 |
| 0xA92356AE | Max roads funding percent | f32 | 120 |
| 0xA92356AF | Max mass transit funding percent | f32 | 120 |
| 0xA92356B0 | Damaged road extra step cost | f32 | 0.1 |
| 0xA92356B1 | Income per tile by travel type | f32 | 0, 0, 0.001, 0.001, 0, 0, 0.001, 0.001, 0.001 |
| 0xA92356B2 | Mass Transit Usage Chance | u8 | 100 |
| 0xA92356B3 | Network Traffic Capacity | f32 | 1000, 3000, 4000, 100, 0, 0, 2500, 3000, 3000, 3000, 2000, 0, 4000 |
| 0xA92356B4 | Travel type generates traffic | bool | False, True, False, True, True, True, True, True, True |
| 0xA92356B5 | Travel type can reach destination | bool | True, True, False, False, True, True, False, False, False |
| 0xA92356B8 | Maximum distance from origin to network | u32 | 1 |
| 0xA92356B9 | Congestion vs Speed | f32 | 0, 1, 1, 1, 2, 0.65, 3, 0.3 |
| 0xA92356BA | Commute trip max time | f32 | 6 |
| 0xA92356BB | Intersection and Turn Capacity Effect | f32 | 0.7, 0.8, 0.9 |
| 0xA92356BC | Trip Starting Cost by travel type | f32 | 0, 0.4, 0, 0, 0, 0, 0, 0, 0 |
| 0xA92356BD | Job Scaling Constant | f32 | 1.2 |
| 0xA92356BE | Population Background Traffic | f32 | 0.05, 0.2, 0.2 |
| 0xA92356BF | Travel type affected by traffic | bool | False, True, True, True, True, True, True, True, False |
| 0xCA5F7821 | Transit Switch Entry Cost vs. Budget | f32 | 0, 2, 100, 1 |
| 0xCA76013B | Trip Length to Minutes Display Multiplier | f32 | 25 |
| 0xCAD64136 | Trip Starting Cost by travel type for Car Pref | f32 | 0.1, 0, 0, 0, 0, 0, 0, 0, 0 |
| 0xEA7B5F06 | Max Mass Transit Strategy Trip Length | f32 | 4 |
| 0xEA8C3CDB | Trip Starting Cost by travel type for Mass Transit | f32 | 0, 1.95, 0, 0, 0, 0, 0, 0, 0 |

## 0xE7E2C8D8 — Land Value Sim  (SimCity_1.dat)

| Property | Name | Type | Value |
|---|---|---|---|
| 0x00000010 |  | u32 | 5 |
| 0x00000020 |  | string | Land Value Sim |
| 0x47E2C200 | Tract Size | u8 | 2 |
| 0x47E2C300 | Land Value Intrinsic Min/Max | u8 | 1, 64 |
| 0x47E2C301 | Land Value Wealth Boundaries | u8 | 70, 120 |
| 0x47E2C310 | Land Value Water Effect | u8 | 50, 10 |
| 0x47E2C320 | Land Value Altitude Effect | f32 | 0, 0, 25, 5, 50, 10, 75, 21, 100, 50, 125, 30, 150, 0 |
| 0x47E2C330 | Land Value Desirability IDs | u32 | 4144 |
| 0x47E2C331 | Land Value Desirability Factors | f32 | 2.5 |
| 0x47E2C332 | Land Value Desirability Curve | f32 | -255, -255, 255, 255 |

## 0xE8DA7677 — Building Development  (SimCity_1.dat)

| Property | Name | Type | Value |
|---|---|---|---|
| 0x00000010 |  | u32 | 5 |
| 0x00000020 |  | string | Building Development |
| 0x2A2D7824 | Lot Developer: Cost Multiplier vs lot slope | f32 | 0, 0, 0.555556, 10, 1.11111, 20, 1.66667, 40, 2.22222, 80, 2.77778, 170, 3.33333, 350, 3.88889, 720, 4.44445, 1470, 5, 3000 |
| 0x2AB845E7 | Lot Developer: Cost to Demolish Destroyed Tile | u32 | 2 |
| 0x47E2C380 | Construction Resources Min/Max | u32 | 8192, 20480 |
| 0x47E2C383 | Stage Popluation Multiplier | f32 | 1.2, 1.1, 1 |
| 0x47E2C400 | Lot Developer Stay New Time | u16 | 90 |
| 0x47E2C401 | Lot Developer Occupancy Thresholds | f32 | 0.3, 0.45, 0.5 |
| 0x47E2C500 | Tract Developer Overbuild | f32 | 1.5 |
| 0x47E2C501 | Tract Developer Reoccupy Thresholds | f32 | 0, 1.2 |
| 0x47E2C502 | Tract Developer Redevelop Size Threshold | f32 | 1.5 |
| 0x47E2C510 | Tract Developer Demolition Costs | f32 | 1, 0.2 |
| 0x47E2C511 | Tract Developer New Lot Costs | f32 | 1, 1 |
| 0x47E2C530 | Tract Developer Occupancy Variation | f32 | 0 |
| 0x47E2C531 | Tract Developer Min Vacancy Range | f32 | 0.01, 0.1 |
| 0x47E2C540 | Tract Developer Kick Out Lower Wealth | bool | True |
| 0x47E2C550 | Tract Developer Available Styles | u32 | 8192, 8193, 8194, 8195 |
| 0x47E2C551 | Tract Developer Years Between Styles | s32 | 5 |
| 0xE900EA24 | kMaxCellAltitudeDeltaForFlatnessProperty | f32 | 1 |
| 0xE900EA25 | kMaxSlopeForFlatnessProperty | f32 | 1 |
| 0xE900EA26 | kDefaultLotFoundationTypeProperty | u32 | 2299228946 |
| 0xE900EA27 | kShouldAttemptLotLevellingProperty | u32 | 1 |
| 0xE900EA28 | kShouldAttemptInternalLotLevellingProperty | u32 | 1 |
| 0xE900EA29 | kLotAutoLevelInternalAltitudeDeltaProperty | f32 | 20 |
| 0xE900EA2A | MaxAreaSlopeAllowedForLevellingProperty | f32 | 40 |
| 0xE900EA2B | Lot Developer: Night Update Interval | u32 | 100 |
| 0xE900EA2C | Lot Developer: Night Update Count | u32 | 10 |
| 0xE900EA2D |  | u32 | 1 |

## 0xE9482490 — Zone Manager  (SimCity_1.dat)

| Property | Name | Type | Value |
|---|---|---|---|
| 0x00000010 |  | u32 | 5 |
| 0x00000020 |  | string | Zone Manager |
| 0xE94825B2 | kZM_mZoneMinCellSize | s32 | 0, 1, 1, 1, 1, 1, 1, 4, 1, 1, 1, 1, 1, 1, 2, 1 |
| 0xE94825B3 | kZM_mZoneMaxCellSize | s32 | 32768, 32, 32, 32, 32, 32, 32, 48, 32, 32, 32, 32, 32, 32, 32, 32 |
| 0xE94825B4 | kZM_mZoneCreationCostArray | s64 | 0, 10, 20, 50, 10, 20, 50, 10, 20, 50, 1, 1, 1, 1, 50, 1 |
| 0xE94825B5 | kZM_mZoneDestructionCostArray | s64 | 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1 |
| 0xE94825B6 | kZM_mZoneTextureResourceInstanceArray | u32 | 4121, 4117, 4118, 4116, 4099, 4100, 4098, 4112, 4113, 4111, 4114, 4096, 4120, 4123, 4101, 4122 |
| 0xE94825B7 | Zone Display Alpha | u8 | 170 |
| 0xE94825B8 | Zone Drag Color | u32 | 4278190335, 12845311, 10092799, 7471359, 1333264383, 542375935, 337699583, 4292555519, 4289868799, 3465551871, 0, 0, 0, 0, 0, 0 |
| 0xE94825B9 | Zone Max Slope | f32 | 256, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16 |

## 0xE9590E52 — Main Simulator  (SimCity_1.dat)

| Property | Name | Type | Value |
|---|---|---|---|
| 0x00000010 |  | u32 | 5 |
| 0x00000020 |  | string | Main Simulator |
| 0x09B73421 | k24HourClockNightBeginProperty | u32 | 20 |
| 0x09B73422 | k24HourClockNightEndProperty | u32 | 1 |
| 0x89590E6D | Simulator Speeds | u32 | 2000, 1000, 50 |
| 0x89590E6E | kAnimationTimeDilationProperty | f32 | 1, 1, 1.5, 3, 1 |
| 0x897E709F | 24 Hour Clock speed (time magnification) | f32 | 135 |
