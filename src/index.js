ws = new WebSocket("ws://192.168.0.100:50079");
console.log(ws);

const url_arg_str = window.location.search;
const url_params = new URLSearchParams(url_arg_str);
const subid = Number(url_params.get('subid'));
console.log(subid);


ws.onopen = (event) => {

    if (subid) {
	var byte_array = new Uint8Array(1);
	byte_array[0] = subid % 256;
	ws.send(byte_array);
    }
    
    ws.addEventListener('message', (event) => {
	console.log('Msg Frm Srv: ', event.data);
    });
    /*
      setTimeout(() => ws.send("msg A"), 1000);
      setTimeout(() => ws.send("msg B"), 1000);
    */
    ws.send("msg A");
    ws.send("msg B");
    
    setTimeout(() => ws.send("Hey now"), 1000);
    setTimeout(() => ws.send("brown cow"), 2000);
    setTimeout(() => ws.send("ab"), 3000);
    setTimeout(() => ws.send("cd goldfish"), 4000);
    setTimeout(() => ws.send("lmno goldfish"), 5000);
    setTimeout(() => ws.send("osmr"), 6000);
    setTimeout(() => ws.send("Ok I'm Done."), 7000);
}
