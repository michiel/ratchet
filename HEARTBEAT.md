 - Add a job output to stdio
 - create a heartbeat task that has main.js returning {"status": "ok"}'
 - including this heartbeat task in registry by default, embed it in the ratchet binary
 - by default, add a schedule that runs the heartbeat task every 5 minutes

