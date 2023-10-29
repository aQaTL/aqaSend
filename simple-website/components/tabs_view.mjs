const BUTTON_CLASS = "tabsview-button";
const ACTIVE_BUTTON_CLASS = "tabsview-button-active";
const RIBBON_CLASS = "tabsview-ribbon";
const HIDDEN_CLASS = "tabsview-hidden";

const TAB_ATTRIBUTE = "tab";
const ACTIVE_TAB_ATTRIBUTE = "tab-active";

/**
 * Tabbed view.
 *
 * Define tabs by adding `tab="tabName"` attributes to outermost child elements.
 * Add `tab-active` attribute to the element that should be opened by default.
 *
 * Example:
 * <tabs-view>
 * 	<div tab="Upload" tab-active>
 * 		<form>
 * 			<p>This is my upload form</p>
 * 		</form>
 * 	</div>
 *
 * 	<div tab="Items">
 * 		<ol>
 * 			<li>This</li>
 * 			<li>is</li>
 * 			<li>my</li>
 * 			<li>list</li>
 * 			<li>of</li>
 * 			<li>uploaded</li>
 * 			<li>items</li>
 * 		</ol>
 * 	</div>
 * </tabs-view>
 */
export default class TabsView extends HTMLElement {
	/**
	 * @typedef TabButtonAndContent
	 * @type {object}
	 * @property {HTMLButtonElement} button
	 * @property {Element} content
	 */

	/**@type{Map<string, TabButtonAndContent>}*/
	#tabContents = new Map();

	/**@type{TabButtonAndContent}*/
	activeTab;

	constructor() {
		super();

		const tabs = document.createElement("div");

		const style = document.createElement("style");
		style.innerHTML = `.${HIDDEN_CLASS} { display: none !important; }`;
		tabs.appendChild(style);

		const tabsRibbon = document.createElement("div");
		tabsRibbon.className = "tabsview-ribbon";

		const contentElements = this.querySelectorAll("[tab]");
		// @ts-ignore //All browsers support iterator on NodeList
		for (const child of contentElements) {
			const tabName = child.getAttribute(TAB_ATTRIBUTE);

			const button = document.createElement("button");
			button.setAttribute("type", "button");
			button.className = BUTTON_CLASS;
			button.addEventListener("click", (_event) => {
				this.openTab(tabName);
			});
			button.append(tabName);

			tabsRibbon.appendChild(button);

			/**@type{TabButtonAndContent}*/
			const tabButtonAndContent = {
				button: button,
				content: child,
			};

			if (child.hasAttribute(ACTIVE_TAB_ATTRIBUTE)) {
				button.classList.add(ACTIVE_BUTTON_CLASS);
				this.activeTab = tabButtonAndContent;
			} else {
				child.classList.add(HIDDEN_CLASS);
			}

			this.#tabContents.set(tabName, tabButtonAndContent);
		}
		tabs.appendChild(tabsRibbon);

		this.insertBefore(tabs, this.firstChild);
	}

	/* Methods */

	/**
	 *
	 * @param tabName {string}
	 */
	openTab(tabName) {
		for (const [_tabName, tab] of this.#tabContents) {
			tab.content.classList.add(HIDDEN_CLASS)
			tab.button.classList.remove(ACTIVE_BUTTON_CLASS)
		}
		const activeTab = this.#tabContents.get(tabName);
		activeTab.content.classList.remove(HIDDEN_CLASS);
		activeTab.button.classList.add(ACTIVE_BUTTON_CLASS);
		this.activeTab = activeTab;
	}

	/**
	 *
	 * @param {string} id
	 * @returns {TabsView}
	 */
	static getById(id) {
		return /**@type{TabsView}*/(document.getElementById(id))
	}
};

customElements.define("tabs-view", TabsView);
