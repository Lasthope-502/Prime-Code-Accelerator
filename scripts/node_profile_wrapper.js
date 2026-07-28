const inspector = require('inspector');
const fs = require('fs');
const path = require('path');

const session = new inspector.Session();
session.connect();

const targetScript = process.argv[2];
const targetArgs = process.argv.slice(3);
process.argv = [process.argv[0], targetScript, ...targetArgs];

session.post('Profiler.setSamplingInterval', { interval: 100 }, () => {
  session.post('Profiler.enable', () => {
    session.post('Profiler.start', () => {
      try {
        require(path.resolve(targetScript));
      } catch (e) {
        console.error(e);
      }

      setImmediate(() => {
        session.post('Profiler.stop', (err, { profile }) => {
          if (!err) {
            const results = aggregateProfile(profile);
            fs.writeFileSync('accel_node_profile.json', JSON.stringify(results.slice(0, 50), null, 2));
          }
          session.disconnect();
        });
      });
    });
  });
});

function aggregateProfile(profile) {
  const agg = new Map();
  const intervalMs = 0.1;

  for (const node of profile.nodes) {
    if (!node.hitCount || node.hitCount <= 0) continue;
    const cf = node.callFrame;
    const key = `${cf.functionName || '(anonymous)'}|${cf.url}|${cf.lineNumber + 1}`;

    if (!agg.has(key)) {
      agg.set(key, {
        function: cf.functionName || '(anonymous)',
        filename: cf.url || '(unknown)',
        line: cf.lineNumber + 1,
        hit_count: 0,
        estimated_self_time_ms: 0,
      });
    }
    const entry = agg.get(key);
    entry.hit_count += node.hitCount;
    entry.estimated_self_time_ms += node.hitCount * intervalMs;
  }

  return Array.from(agg.values()).sort((a, b) => b.hit_count - a.hit_count);
}