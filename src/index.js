ws = new WebSocket("ws://192.168.0.100:3333");
console.log(ws);

ws.onopen = (event) => {
    ws.send("A message ey");
    ws.send("And another");
}
