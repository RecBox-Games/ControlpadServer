ws = new WebSocket("ws://192.168.0.100:50079");
console.log(ws);



ws.onopen = (event) => {
    ws.addEventListener('message', (event) => {
	console.log('Msg Frm Srv: ', event.data);
    });
    /*
      setTimeout(() => ws.send("msg A"), 1000);
      setTimeout(() => ws.send("msg B"), 1000);
    */
    ws.send("msg A");
    ws.send("msg B");
    
    /*setTimeout(() => ws.send("Hey now"), 1000);
    setTimeout(() => ws.send("brown cow"), 1000);
    setTimeout(() => ws.send("ab"), 1000);
    setTimeout(() => ws.send("cd goldfish"), 1000);
    setTimeout(() => ws.send("lmno goldfish"), 1000);
    setTimeout(() => ws.send("osmr"), 1000);
    setTimeout(() => ws.send("Ok I'm Done."), 1000);*/
}
