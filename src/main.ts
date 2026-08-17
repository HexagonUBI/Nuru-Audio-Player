import { mount } from 'svelte';
import './styles/base.css';
import App from './App.svelte';

// The window is created hidden and shown from Rust once the WebView has painted,
// so there is no white flash on launch. Nothing to do here but mount.
export default mount(App, { target: document.getElementById('app')! });
