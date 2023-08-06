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

## Error handling

- [x] Better error type for handling fallible http requests
    - [x] Separation between internal and external errors
    - [x] User presentable toggle
    - [x] Various error formats (plaintext, json, http)
    - [x] Ease of composability (not needing to add Http and Hyper errors everytime)

# Website

- [x] Log in page
- [x] Account page 
  - [ ] Generate registration link
- [ ] Registration page 
  - no button on the website, accessible by registration link
- [ ] My files page
- [ ] Display username of currently logged-in user
