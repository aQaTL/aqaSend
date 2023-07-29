"use strict";

import * as Types from "./models.mjs"
import { API_SERVER } from "./api.mjs";

function hello() {
	let greetingEl = document.getElementById("greeting");
	greetingEl.innerHTML = "All files";
}

window.addEventListener("DOMContentLoaded", function(_event) {
	hello();
	loadFiles();
});

function loadFiles() {
	console.log("Loading files");

	const request = new XMLHttpRequest();
	request.addEventListener("load", (event) => {
		/** @type [Types.FileModel] */
		const response = event.target.response;
		console.log("success: " + JSON.stringify(response));

		displayFiles(response);
	});

	request.responseType = "json";
	request.open("GET", `${API_SERVER}/api/list.json`);
	request.send();
}

/**
 * Renders file list 
 * @param {[Types.FileModel]} files - list of files
 */
function displayFiles(files) {
	let html = "";
	let mainEl = document.getElementsByTagName("main")[0];
	for (let i = 0; i < files.length; i++) {
		const file = files[i];

		let fileEntryEl = document.createElement("div");

		let fileLinkEl = document.createElement("a");
		fileLinkEl.href = `${API_SERVER}/api/download/${file.id}`;
		fileLinkEl.appendChild(document.createTextNode(file.filename));
		fileEntryEl.appendChild(fileLinkEl);

		let statsEl = document.createElement("div");
		{
			let div = document.createElement("div");
			div.appendChild(document.createTextNode("lifetime: " + file.lifetime));
			statsEl.appendChild(div);
		}
		{
			let div = document.createElement("div");
			div.appendChild(document.createTextNode("visibility: " + file.visibility));
			statsEl.appendChild(div);
		}
		{
			let div = document.createElement("div");
			div.appendChild(document.createTextNode(
				"upload date: " + JSON.stringify(file.upload_date)
			));
			statsEl.appendChild(div);
		}
		{
			let div = document.createElement("div");
			div.appendChild(document.createTextNode(
				"download count: " + file.download_count
			));
			statsEl.appendChild(div);
		}
		{
			let div = document.createElement("div");
			div.appendChild(document.createTextNode(
				"download count type: " + JSON.stringify(file.download_count_type)
			));
			statsEl.appendChild(div);
		}
		fileEntryEl.appendChild(statsEl);

		mainEl.appendChild(fileEntryEl);
	}

}
