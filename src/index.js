ws = new WebSocket("ws://192.168.0.100:50079");
console.log(ws);

ws.onopen = (event) => {
    setTimeout(() => ws.send("A message ey"), 1000);
    setTimeout(() => ws.send("A message bee"), 1000);
    setTimeout(() => ws.send("Hey now"), 1000);
    setTimeout(() => ws.send("brown cow"), 1000);
    setTimeout(() => ws.send("ab"), 1000);
    setTimeout(() => ws.send("cd goldfish"), 1000);
    setTimeout(() => ws.send("lmno goldfish"), 1000);
    setTimeout(() => ws.send("osmr"), 1000);
    setTimeout(() => ws.send("Ok I'm Done."), 1000);
}
