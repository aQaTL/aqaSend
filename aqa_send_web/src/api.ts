export const API_ENDPOINT =
	process.env.NODE_ENV === "production" ? "notsureyet" : "http://127.0.0.1:8000";

export type DownloadCount =
	"infinite"
	| "1"
	| "5"
	| "10"
	| "100";

export enum Visibility {
	public = "public",
	private = "private",
}

export type Lifetime = "infinite" | string;

export interface UploadParams {
	visibility: Visibility,
	downloadCount: DownloadCount,
	password: string | "none",
	lifetime: Lifetime,
}

export async function uploadFile(file: Blob,
								 filename: string,
								 params: UploadParams
): Promise<boolean>
{
	console.debug(`Uploading ${filename}`);

	let form = document.createElement("form");
	form.enctype = "multipart/form-data";

	let formData = new FormData(form);
	formData.append("file", file, filename);

	try {
		let response = await fetch(`${API_ENDPOINT}/api/upload`, {
			method: "POST",
			headers: {
				"aqa-visibility": params.visibility,
				"aqa-download-count": params.downloadCount,
				"aqa-password": params.password,
				"aqa-lifetime": params.lifetime,
			},
			body: formData,
		});

		console.log("Response: ", response);
		return response.status == 200;
	} catch (ex) {
		console.error(ex);
		return false;
	}
}