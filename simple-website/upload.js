"use strict";

import * as Types from "./models.mjs";
import {API_SERVER} from "./api_endpoint.mjs";
import * as Api from "./api.mjs";

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

	const fileInputEl = submitFormEl["file"]
	const formData = new FormData();
	for (let i = 0; i < fileInputEl.files.length; i++) {
		formData.append("file", fileInputEl.files[i], fileInputEl.files[i].name);
	}

	const request = new XMLHttpRequest();
	request.addEventListener("load", (_event) => {
		if (request.status === 200) {
			/** @type {[Types.UploadedFile]} */
			const response = request.response;

			console.log("success: " + JSON.stringify(response));
			displayInfoMsg(`Successfully uploaded ${response.length} files`, UPLOAD_RESULT_SUCCESS);
		} else {
			/** @type {Types.ErrorJsonBody} */
			const response = request.response;

			console.error("error: " + JSON.stringify(response));
			displayInfoMsg(`Upload failed: ${response.message}`, UPLOAD_RESULT_FAILURE);
		}
	});

	request.addEventListener("error", (_event) => {
		/** @type {Types.ErrorJsonBody} */
		const response = request.response;

		console.error("error: " + JSON.stringify(response));
		displayInfoMsg(`Upload failed: ${response.message}`, UPLOAD_RESULT_FAILURE);
	});


	request.responseType = "json";
	request.open("POST", `${API_SERVER}/api/upload`);
	request.setRequestHeader("aqa-download-count", downloadCount);
	request.setRequestHeader("aqa-lifetime", lifetime);
	request.setRequestHeader("aqa-visibility", visibility);
	if (password.trim().length !== 0) {
		request.setRequestHeader("aqa-password", encodeURIComponent(password));
	}
	hideInfoMsg();
	request.send(formData);
}

/**
 * @param {FileList} files
 */
function updateFileList(files) {
	let fileUploadFormSectionEl = /**@type{HTMLDivElement}*/(document.getElementById("fileUploadFormSection"));;

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

const UPLOAD_RESULT_SUCCESS = 0;
const UPLOAD_RESULT_FAILURE = 1;

/**
 * Displays a block with info operation result.
 *
 * @param {string} msg - Message to display in the box
 * @param {number} result - One of: [UPLOAD_RESULT_SUCCESS, UPLOAD_RESULT_FAILURE]
 */
function displayInfoMsg(msg, result) {
	let infoMsgEl = document.getElementById("infoMsg");
	switch (result) {
		case UPLOAD_RESULT_SUCCESS: {
			infoMsgEl.style.display = "block";
			infoMsgEl.className = "infoMsgSuccess";
			infoMsgEl.innerText = msg;

		}
			break;
		case UPLOAD_RESULT_FAILURE: {
			infoMsgEl.style.display = "block";
			infoMsgEl.className = "infoMsgFailure";
			infoMsgEl.innerText = msg;

		}
			break;
	}
}

/**
 * Hides the infoMsg box
 */
function hideInfoMsg() {
	let infoMsgEl = document.getElementById("infoMsg");
	infoMsgEl.style.display = "none";
}
