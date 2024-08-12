/*
 * Copyright 2022-2024 RecBox, Inc.
 *
 * This file is part of the ControlpadServer program of the GameNite project.
 *
 * ControlpadServer is free software: you can redistribute it and/or modify it 
 * under the terms of the GNU General Public License as published by the Free 
 * Software Foundation, either version 3 of the License, or (at your option) 
 * any later version.
 * 
 * ControlpadServer is distributed in the hope that it will be useful, but 
 * WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY 
 * or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for 
 * more details.
 * 
 * You should have received a copy of the GNU General Public License along with 
 * ControlpadServer. If not, see <https://www.gnu.org/licenses/>.
 */

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
