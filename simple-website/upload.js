"use strict";

import * as Types from "./models.mjs";
import {API_SERVER} from "./api_endpoint.mjs";
import * as Api from "./api.mjs";
import InfoMsgBox from "./components/info_msg_box.mjs";
import TabsView from "./components/tabs_view.mjs";

function main() {
	let uploadFormEl = document.getElementById("uploadForm");
	uploadFormEl.addEventListener("submit", submitUploadForm);

	let fileEl = /**@type{HTMLInputElement}*/(document.getElementById("file"));
	fileEl.addEventListener("change", (_event) => {
		updateFileList(fileEl.files);
	});
}

async function loadUser() {
	let username = /** @type {?string} */ (await Api.loadUser());
	if (username) {
		console.log(`current user: ${username}`);
		let userInfoEl = document.getElementById("userInfo");
		userInfoEl.innerText = username;
		userInfoEl.style.display = "block";
	} else {
		console.log(`No user logged in`);
	}
}

window.addEventListener("DOMContentLoaded", function (_event) {
	main();
	loadUser();
});

/**
 * Tries to submit the upload form
 *
 * @param {SubmitEvent} event - Event fired when clicked on submit
 */
function submitUploadForm(event) {
	event.preventDefault();

	console.log("Clicked to submit!");

	const submitFormEl = event.target;

	const visibility = submitFormEl["visibility-select"].value;
	const downloadCount = submitFormEl["download-count-select"].value;
	const lifetime = submitFormEl["lifetime-select"].value;
	/** @type string */
	const password = submitFormEl["password"].value;

	const tabsView = TabsView.getById("uploadTypeTabs");

	const formData = new FormData();

	if (tabsView.activeTab.content.id === "fileUploadFormSection") {
		const fileInputEl = submitFormEl["file"]
		for (let i = 0; i < fileInputEl.files.length; i++) {
			formData.append("file", fileInputEl.files[i], fileInputEl.files[i].name);
		}
	} else if (tabsView.activeTab.content.id === "text-upload") {
		const textInputEl = submitFormEl["text-box"];
		const fileNameEl = submitFormEl["text-filename"];

		// Encode the text as UTF-8
		const encoder = new TextEncoder();
		const view = encoder.encode(textInputEl.value);
		let blob = new Blob([view], { type: "text/plain" });

		formData.append("file", blob, fileNameEl.value);
	}

	const request = new XMLHttpRequest();
	const resultBox = InfoMsgBox.getById("uploadResult");

	request.addEventListener("load", (_event) => {
		if (request.status === 200) {
			/** @type {[Types.UploadedFile]} */
			const response = request.response;

			console.log("success: " + JSON.stringify(response));
			resultBox.displaySuccess(`Successfully uploaded ${response.length} files`);
		} else {
			/** @type {Types.ErrorJsonBody} */
			const response = request.response;

			console.error("error: " + JSON.stringify(response));
			resultBox.displayFailure(`Upload failed: ${response.message}`);
		}
	});

	request.addEventListener("error", (_event) => {
		/** @type {Types.ErrorJsonBody} */
		const response = request.response;

		console.error("error: " + response && JSON.stringify(response) || "");
		resultBox.displayFailure(`Upload failed`);
	});

	request.upload.addEventListener("progress", (event) => {
		const percentage = Math.round((event.loaded * 100.0 / event.total) || 100);
		resultBox.displaySuccess(`Uploading... ${percentage}%`);
	});

	request.responseType = "json";
	request.open("POST", `${API_SERVER}/api/upload`);
	request.setRequestHeader("aqa-download-count", downloadCount);
	request.setRequestHeader("aqa-lifetime", lifetime);
	request.setRequestHeader("aqa-visibility", visibility);
	if (password.trim().length !== 0) {
		request.setRequestHeader("aqa-password", encodeURIComponent(password));
	}
	resultBox.hide();
	request.send(formData);
}

/**
 * @param {FileList} files
 */
function updateFileList(files) {
	let fileUploadFormSectionEl = /**@type{HTMLDivElement}*/(document.
		getElementById("fileUploadFormSection"));;

	/**@type{?HTMLElement}*/
	let fileListSectionEl = null;
	if (fileUploadFormSectionEl.nextElementSibling.id === "selectedFilesListFormSection") {
		fileListSectionEl = document.getElementById("selectedFilesListFormSection");
	} else {
		fileListSectionEl = document.createElement("section");
		fileListSectionEl.id = "selectedFilesListFormSection";
		fileListSectionEl.className = "uploadFormSection";

		let textDiv = document.createElement("div");
		textDiv.append("Selected files:")
		textDiv.appendChild(document.createElement("ul"))
		fileListSectionEl.appendChild(textDiv);

		fileUploadFormSectionEl.insertAdjacentElement("afterend", fileListSectionEl);
	}

	let fileListEl = /**@type{HTMLUListElement}*/(fileListSectionEl.querySelector("ul"));

	while (fileListEl.firstChild) {
		fileListEl.removeChild(fileListEl.lastChild);
	}

	for (let i = 0; i < files.length; i++) {
		let file = files[i];

		let fileNameEl = document.createElement("div");
		fileNameEl.appendChild(document.createTextNode(file.name));

		fileListEl.appendChild(fileNameEl);
	}
}
