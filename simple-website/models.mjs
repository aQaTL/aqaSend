/**
 * @typedef ErrorJsonBody
 * @type {object}
 * @property {number} status - HTTP status code
 * @property {string} message - error message
*/

/**
 * @typedef UploadedFile
 * @type {object}
 * @property {string} uuid -
 * @property {string} filename -
*/

/**
 * @typedef FileModel
 * @type {object}
 * @property {!string} uuid - file uuid

 * @property {!string} filename -
 * @property {!string} content_type -
 * @property {!string} uploader_uuid -
 * 
 * @property {!number} download_count -
 *
 * @property {!('Public'|'Private')} visibility -
 * @property {!boolean} has_password -
 *
 * @property {?Duration} lifetime -
 * @property {!Date} upload_date -
*/

/**
* @typedef Date
* @property {number} secs_since_epoch -
* @property {number} nanos_since_epoch - 
*/

/**
 * @typedef Duration
 * @type {object}
 * @property {number} secs -
 * @property {number} nanos -
 */

/**
 * @typedef CreateAccountResponse
 * @type {object}
 * @property {string} uuid -
 */


export const Types = {};
