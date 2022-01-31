<template>
  <div id="nav">
    <h1>aQaSend</h1>
    <router-link to="/">Home</router-link> |
    <router-link to="/upload">Upload</router-link>
    <router-link to="/account">Account</router-link>
  </div>
  <router-view/>
</template>

<style lang="scss">
body {
  background-color: #2c3e50;
  margin-top: 0;
}

#app {
  font-family: Avenir, Helvetica, Arial, sans-serif;
  text-align: center;
  color: #f59fd8;
}

#nav {
  display: flex;
  flex-direction: row;
  align-items: baseline;
  justify-content: flex-start;
  gap: 10px;

  a {
    color: #f5c6d8;

    font-weight: bold;

    border-style: solid;
    border-radius: 5px;
    border-width: 16px 8px 8px 8px;
    border-color: transparent;

    &.router-link-exact-active {
      background-color: #233240;
      border-color: #233240;
    }
  }
}
</style>
