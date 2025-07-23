import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
/** @type {import('@sveltejs/kit').Config} */
const config = {
	preprocess: vitePreprocess(),
	kit: {
		// For the tutorial, we need to disable CSRF protection.
		// Don't do this in your own apps unless you know what you're doing!
		// See https://kit.svelte.dev/docs/configuration#csrf for more info.
		adapter: adapter(),
		csrf: false
	},
	 onwarn: (warning, handler) => {
    	if (warning.code === 'a11y-click-events-have-key-events') return
    	handler(warning)
  	},

	vitePlugin: {
		// This enables compile-time warnings to be
		// visible in the learn.svelte.dev editor
		onwarn: (warning, defaultHandler) => {
			console.log('svelte:warnings:%s', JSON.stringify(warning));
			defaultHandler(warning);
		}
	}
};

export default config;

