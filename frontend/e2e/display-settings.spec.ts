import { expect, gotoApp, test } from './fixtures';
import { addSystem, createMap } from './arrange';

// Account-wide display preferences: kept on the account, so a change follows the pilot to
// every map.

const J122515 = 31001882; // C5, Wolf-Rayet Star

test('effect icons can be swapped for letters, and back', async ({ page, api }) => {
	// The suite shares one account, so whatever this test sets is put back afterwards.
	await api.post('/api/me/preferences', { data: { effect_icons: true } });
	const mapId = await createMap(api, 'E2E Effect Letters');
	await addSystem(api, mapId, J122515, { x: 200, y: 200 });

	try {
		await gotoApp(page, '/settings/display');
		const toggle = page.getByTestId('effect-icons-switch');
		await expect(toggle).toBeChecked();
		await toggle.click();
		await expect(toggle).not.toBeChecked();

		await gotoApp(page, `/maps/${mapId}`);
		const badge = page
			.getByTestId('system-node')
			.filter({ hasText: 'J122515' })
			.getByLabel('Wolf-Rayet Star');
		await expect(badge).toHaveText('W');
		await expect(badge.locator('img')).toHaveCount(0);

		// The setting is the account's, so the API says the same as the page did.
		const saved = await api.get('/api/me/preferences');
		expect(await saved.json()).toEqual({ effect_icons: false, route_previews: true });

		await gotoApp(page, '/settings/display');
		await page.getByTestId('effect-icons-switch').click();
		await expect(page.getByTestId('effect-icons-switch')).toBeChecked();
		await gotoApp(page, `/maps/${mapId}`);
		await expect(badge.locator('img')).toBeVisible();
	} finally {
		await api.post('/api/me/preferences', { data: { effect_icons: true } });
		await api.delete(`/api/maps/${mapId}`);
	}
});

const JITA = 30000142;

test('routes in lists can be turned off, from the map or the settings page', async ({
	page,
	api,
}) => {
	// The suite shares one account, so whatever this test sets is put back afterwards.
	await api.post('/api/me/preferences', { data: { route_previews: true } });
	const mapId = await createMap(api, 'E2E Route Previews');
	await addSystem(api, mapId, JITA, { x: 200, y: 200 });

	try {
		// Wide enough that the watchlist has the room to draw routes.
		await page.setViewportSize({ width: 2400, height: 1200 });
		await gotoApp(page, `/maps/${mapId}`);
		await page.getByTestId('system-node').filter({ hasText: 'Jita' }).click();
		const amarr = page.getByTestId('watchlist-row').filter({ hasText: 'Amarr' });
		await expect(amarr.getByTestId('route-strip')).toBeVisible();

		await page.getByTestId('route-settings').click();
		await page.getByTestId('setting-route-previews').click();
		await expect(page.getByTestId('route-strip')).toHaveCount(0);
		// The count is still there, and is still what opens the route.
		await expect(amarr.getByTestId('jump-badge')).toHaveText('11j');

		await gotoApp(page, '/settings/display');
		const toggle = page.getByTestId('route-previews-switch');
		await expect(toggle).not.toBeChecked();
		await toggle.click();
		await expect(toggle).toBeChecked();
		await gotoApp(page, `/maps/${mapId}`);
		await page.getByTestId('system-node').filter({ hasText: 'Jita' }).click();
		await expect(amarr.getByTestId('route-strip')).toBeVisible();
	} finally {
		await api.post('/api/me/preferences', { data: { route_previews: true } });
		await api.delete(`/api/maps/${mapId}`);
	}
});
