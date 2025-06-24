import { beforeAll, describeSuite, expect } from "@moonwall/cli";
import type { ApiPromise } from "@polkadot/api";

describeSuite({
    id: "SMOK13",
    title: "Smoke test for pooled staking holds consistency",
    foundationMethods: "read_only",
    testCases: ({ it, context, log }) => {
        let api: ApiPromise;

        beforeAll(async () => {
            api = context.polkadotJs();
        });

        it({
            id: "C01",
            title: "Sum of holds across all delegations matches total delegated amount",
            timeout: 120000,
            test: async () => {
                // Get all candidates
                const sortedCandidates = await api.query.pooledStaking.sortedEligibleCandidates();
                
                // Track total holds per delegator
                const delegatorTotalHolds = new Map<string, bigint>();
                
                // Track total staked amounts across all pools
                let totalStakedAcrossAllPools = 0n;
                
                log(`Checking holds consistency for ${sortedCandidates.length} candidates...`);
                
                for (const candidateData of sortedCandidates) {
                    const candidate = candidateData.candidate.toHex();
                    
                    // Get all pool data for this candidate
                    const poolEntries = await api.query.pooledStaking.pools.entries(candidate);
                    
                    for (const [storageKey, value] of poolEntries) {
                        const poolKey = storageKey.args[1];
                        const amount = value.toBigInt();
                        
                        // Check if this is a HeldStake entry
                        if (poolKey && poolKey.toHuman() && typeof poolKey.toHuman() === 'object') {
                            const keyHuman = poolKey.toHuman() as any;
                            
                            // Process held stake entries
                            if (keyHuman.JoiningSharesHeldStake && keyHuman.JoiningSharesHeldStake.delegator) {
                                const delegator = keyHuman.JoiningSharesHeldStake.delegator.toString();
                                const currentHold = delegatorTotalHolds.get(delegator) || 0n;
                                delegatorTotalHolds.set(delegator, currentHold + amount);
                            }
                            else if (keyHuman.AutoCompoundingSharesHeldStake && keyHuman.AutoCompoundingSharesHeldStake.delegator) {
                                const delegator = keyHuman.AutoCompoundingSharesHeldStake.delegator.toString();
                                const currentHold = delegatorTotalHolds.get(delegator) || 0n;
                                delegatorTotalHolds.set(delegator, currentHold + amount);
                            }
                            else if (keyHuman.ManualRewardsSharesHeldStake && keyHuman.ManualRewardsSharesHeldStake.delegator) {
                                const delegator = keyHuman.ManualRewardsSharesHeldStake.delegator.toString();
                                const currentHold = delegatorTotalHolds.get(delegator) || 0n;
                                delegatorTotalHolds.set(delegator, currentHold + amount);
                            }
                            else if (keyHuman.LeavingSharesHeldStake && keyHuman.LeavingSharesHeldStake.delegator) {
                                const delegator = keyHuman.LeavingSharesHeldStake.delegator.toString();
                                const currentHold = delegatorTotalHolds.get(delegator) || 0n;
                                delegatorTotalHolds.set(delegator, currentHold + amount);
                            }
                            // Track total staked amounts
                            else if (keyHuman.JoiningSharesTotalStaked) {
                                totalStakedAcrossAllPools += amount;
                            }
                            else if (keyHuman.AutoCompoundingSharesTotalStaked) {
                                totalStakedAcrossAllPools += amount;
                            }
                            else if (keyHuman.ManualRewardsSharesTotalStaked) {
                                totalStakedAcrossAllPools += amount;
                            }
                            else if (keyHuman.LeavingSharesTotalStaked) {
                                totalStakedAcrossAllPools += amount;
                            }
                        }
                    }
                }
                
                // Now verify that each delegator's total holds match their actual balance holds
                const failures: Array<{delegator: string, totalHolds: bigint, actualHolds: bigint}> = [];
                
                for (const [delegator, totalHolds] of delegatorTotalHolds) {
                    // Query the actual holds on the delegator's balance
                    const actualHolds = await api.query.balances.holds(delegator);
                    
                    // Find the PooledStake hold
                    let pooledStakeHold = 0n;
                    for (const hold of actualHolds) {
                        if (hold.id.toHuman() === 'poolstak0x506f6f6c6564537461' || 
                            hold.id.toHuman() === 'PooledStake' ||
                            hold.id.toHex() === '0x706f6f6c7374616b506f6f6c6564537461') {
                            pooledStakeHold = hold.amount.toBigInt();
                            break;
                        }
                    }
                    
                    if (totalHolds !== pooledStakeHold) {
                        failures.push({
                            delegator,
                            totalHolds,
                            actualHolds: pooledStakeHold
                        });
                    }
                }
                
                // Log any failures
                for (const failure of failures) {
                    console.error(
                        `Hold mismatch for delegator ${failure.delegator}: ` +
                        `Sum of pool holds = ${failure.totalHolds}, ` +
                        `Actual balance hold = ${failure.actualHolds}`
                    );
                }
                
                // Calculate the sum of all holds from our tracking
                const sumOfAllHolds = Array.from(delegatorTotalHolds.values()).reduce((a, b) => a + b, 0n);
                
                // Query all actual balance holds from the balances pallet
                let totalActualHolds = 0n;
                for (const [delegator, _] of delegatorTotalHolds) {
                    const actualHolds = await api.query.balances.holds(delegator);
                    for (const hold of actualHolds) {
                        if (hold.id.toHuman() === 'poolstak0x506f6f6c6564537461' || 
                            hold.id.toHuman() === 'PooledStake' ||
                            hold.id.toHex() === '0x706f6f6c7374616b506f6f6c6564537461') {
                            totalActualHolds += hold.amount.toBigInt();
                            break;
                        }
                    }
                }
                
                log(`Total staked across all pools: ${totalStakedAcrossAllPools}`);
                log(`Sum of all delegator holds (from pools storage): ${sumOfAllHolds}`);
                log(`Sum of all actual balance holds: ${totalActualHolds}`);
                log(`Number of delegators with holds: ${delegatorTotalHolds.size}`);
                
                // Check for any discrepancies
                expect(failures.length, "Found hold mismatches").to.equal(0);
                
                // Verify that the sum from pools storage matches actual balance holds
                expect(
                    sumOfAllHolds,
                    `Sum of pools storage holds (${sumOfAllHolds}) should equal actual balance holds (${totalActualHolds})`
                ).to.equal(totalActualHolds);
                
                // Verify that the total staked matches the sum of holds
                expect(
                    sumOfAllHolds,
                    `Sum of all holds (${sumOfAllHolds}) should equal total staked (${totalStakedAcrossAllPools})`
                ).to.equal(totalStakedAcrossAllPools);
            },
        });
    },
});