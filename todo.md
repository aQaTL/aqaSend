# Server

- [x] Write tests for time based entries
- [x] Write test for infinite download count
- [x] Implement Lifetime header parsing
- [x] Account types
- [x] Admin account
- [x] Cli command to create account
- [x] Log in api
- [x] Add a trait bound `Into<Response<Body>>` to handle_response, so that the error variant
    can also generate a response
- [ ] Have two APIs: JSON (main one) and old school html redirect driven that will use the JSON one
    internally
- [ ] Log out api
- [x] API to generate a registration link
- [x] Creating account from registration code
- [x] Admin and normal user registration code types
- [ ] Delete invite code after 1 use 
- [ ] Websocket API
- [ ] Deleting entries

## Error handling

- [x] Better error type for handling fallible http requests
    - [x] Separation between internal and external errors
    - [x] User presentable toggle
    - [x] Various error formats (plaintext, json, http)
    - [x] Ease of composability (not needing to add Http and Hyper errors everytime)

# Website

- [x] Log in page
    - [ ] Redirect somewhere or display success or refresh the page on successful login 
- [x] Account page 
  - [x] Generate registration link
- [x] Registration page 
  - no button on the website, accessible by registration link
  - [x] Display result of account creation 
- [x] My files page
- [x] Display username of currently logged-in user
- [x] Being able to select whether the registration code should be for an admin or a regular
  account
- [x] Check the registration code when loading the registration page
- [x] Move infoMsg from upload.js into a reusable component (info_msg_box.mjs)
- [x] (upload) Display selected files
- [x] (upload) upload progress
- [ ] Progress spinner and neutral message support for InfoMsgBox 
- [ ] Turn navigation into a web component
- [ ] A page or a section on the upload page with a text box to upload text.  
- [ ] Get update's via WebSockets and refresh the list of entries on the fly
- [ ] Deleting entries
- [ ] Nicer visual indicator of a private entry