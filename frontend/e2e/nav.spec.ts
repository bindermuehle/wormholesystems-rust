import { createdId, expect, gotoApp, test } from './fixtures';

// The top bar: quick access to the maps you keep there, and Tranquility's state.

test('a pinned map shows in the top bar and takes you to it', async ({ page, api }) => {
	const res = await api.post('/api/maps', { data: { name: 'E2E Pinned Map' } });
	const mapId = await createdId(res);

	await gotoApp(page, '/maps');
	const card = page.getByTestId('map-card').filter({ hasText: 'E2E Pinned Map' });
	await card.getByTestId('map-menu').click();
	await page.getByTestId('map-pin').click();

	const shortcut = page.getByTestId('pinned-map').filter({ hasText: 'E2E Pinned Map' });
	await expect(shortcut).toBeVisible();

	await shortcut.click();
	await page.waitForURL(new RegExp(`/maps/(.+-)?${mapId}$`));
	await page.waitForSelector('[data-testid="panel-grid"]');
	// It follows you onto the map, which is the point of a shortcut.
	await expect(page.getByTestId('pinned-map').filter({ hasText: 'E2E Pinned Map' })).toBeVisible();

	// And unpinning takes it back out.
	await gotoApp(page, '/maps');
	await card.getByTestId('map-menu').click();
	await page.getByTestId('map-pin').click();
	await expect(page.getByTestId('pinned-map')).toHaveCount(0);

	await api.delete(`/api/maps/${mapId}`);
});

test('the pilot readout is gone from the bar, and the server status is not', async ({
	page,
	api,
}) => {
	const res = await api.post('/api/maps', { data: { name: 'E2E NavStatus' } });
	const mapId = await createdId(res);
	await gotoApp(page, `/maps/${mapId}`);

	await expect(page.getByTestId('server-status')).toBeVisible();
	// Where you are and what you are flying belong to the map's own status bar.
	await expect(page.getByTestId('status-bar')).toBeVisible();

	await api.delete(`/api/maps/${mapId}`);
});
