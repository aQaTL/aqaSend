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
	let fileEntriesEl = document.getElementById("fileEntries");
	for (let i = 0; i < files.length; i++) {
		const file = files[i];

		let fileEntryEl = document.createElement("div");
		fileEntryEl.className = "fileEntry";

		let fileLinkEl = document.createElement("a");
		fileLinkEl.href = `${API_SERVER}/api/download/${file.id}`;

		let filenameEl = document.createElement("div");
		filenameEl.className = "fileEntryFilename";
		filenameEl.appendChild(document.createTextNode(file.filename));
		fileLinkEl.appendChild(filenameEl);


		if (file.visibility == "Private") {
			fileEntryEl.className += " fileEntryPrivate";
		}

		let statsEl = document.createElement("div");
		{
			let div = document.createElement("div");
			
			let lifetime;
			if ((typeof file.lifetime) == "object") {
				lifetime = formatDuration(file.lifetime.Duration.secs * 1000);
			} else {
				lifetime = file.lifetime;
			}

			div.appendChild(document.createTextNode("lifetime: " + lifetime));
			statsEl.appendChild(div);
		}
		{
			let div = document.createElement("div");
			const uploadDate = new Date(file.upload_date.secs_since_epoch * 1000);
			div.appendChild(document.createTextNode(
				`upload date: ${uploadDate.toLocaleDateString()} ${uploadDate.toLocaleTimeString()}`
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
		fileLinkEl.appendChild(statsEl);

		fileEntryEl.appendChild(fileLinkEl);

		fileEntriesEl.appendChild(fileEntryEl);
	}

}

function formatDuration(ms) {
	const time = {
		day: Math.floor(ms / 86400000),
		hour: Math.floor(ms / 3600000) % 24,
		minute: Math.floor(ms / 60000) % 60,
		second: Math.floor(ms / 1000) % 60,
		millisecond: Math.floor(ms) % 1000
	};
	return Object.entries(time)
		.filter(val => val[1] !== 0)
		.map(([key, val]) => `${val} ${key}${val !== 1 ? "s" : ""}`)
		.join(", ");
}
