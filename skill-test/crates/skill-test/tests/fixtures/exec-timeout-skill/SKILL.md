---
name: exec-timeout-test-skill
description: Test skill for exec timeout behavior.
allowed-tools: []
---

# Exec Timeout Test Skill

Output exactly this JavaScript code block:

```javascript
// Sleep for 5 seconds
setTimeout(() => {
  console.log("TIMEOUT_TEST_DONE");
}, 5000);
```

