<template>
  <div id="nav">
    <h1 id="aqaSendHeader">aQaSend</h1>
    <div id="routerLinks">
      <router-link to="/">Home</router-link>
      <router-link to="/upload">Upload</router-link>
      <router-link to="/account">Account</router-link>
    </div>
  </div>
  <router-view/>
</template>

<style lang="scss">
@import url("./fonts.scss");

body {
  background-color: #2C3E50;
  margin-top: 0;
}

#app {
  font-family: "Nunito Sans", Avenir, Helvetica, Arial, sans-serif;
  text-align: center;
  color: #F59FD8;
}

#nav {
  display: grid;
  grid-template-columns: 1fr 1fr 1fr;
  align-items: baseline;
  border-bottom: 3px dashed #ae9ff5;

  #aqaSendHeader {
    justify-self: start;
  }

  #routerLinks {
    display: flex;
    justify-self: center;
    gap: 10px;
  }

  a {
    color: #F5C6D8;

    font-weight: bold;
    font-size: 14pt;

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
