type CpuUsageSnapshot = {
  user: number;
  system: number;
};

type NodeProcess = {
  cpuUsage: (previousValue?: CpuUsageSnapshot) => CpuUsageSnapshot;
};

function nodeProcess(): NodeProcess {
  const processValue = (globalThis as typeof globalThis & { process?: NodeProcess }).process;
  if (!processValue) {
    throw new Error("CPU performance budgets require the Vitest Node environment.");
  }
  return processValue;
}

export function startCpuMeasurement(): CpuUsageSnapshot {
  return nodeProcess().cpuUsage();
}

export function elapsedCpuMilliseconds(startedAt: CpuUsageSnapshot): number {
  const elapsed = nodeProcess().cpuUsage(startedAt);
  return (elapsed.user + elapsed.system) / 1_000;
}
