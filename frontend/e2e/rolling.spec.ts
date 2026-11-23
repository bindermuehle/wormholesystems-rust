import { createdId, expect, gotoApp, test } from './fixtures';

// The cheat sheet reads the hole masses from the catalog and works the jumps out for the
// ship typed in, so the numbers here are the old quick reference chart's.
test('rolling cheat sheet: steps per hole mass, for the ship typed in', async ({ page, api }) => {
	const mapId = await createdId(await api.post('/api/maps', { data: { name: 'E2E Rolling' } }));
	await gotoApp(page, `/maps/${mapId}`);

	await page.getByTestId('rolling-button').click();
	const dialog = page.getByTestId('rolling-dialog');
	await expect(dialog).toBeVisible();

	// C247 is a 2,000 kt hole: two cold and two hot, then it depends.
	await dialog.getByTestId('rolling-find').fill('c247');
	const hole = dialog.getByTestId('rolling-class');
	await expect(dialog.getByRole('tab', { selected: true })).toHaveText('2,000 kt');
	const flown = (testid: string) => hole.getByTestId(testid).locator('[data-jumps]');
	await expect(flown('rolling-first')).toHaveAttribute('data-jumps', 'cold cold hot hot');
	await expect(flown('rolling-reduced-roll')).toHaveAttribute('data-jumps', 'cold cold hot hot');
	await expect(flown('rolling-fresh-roll')).toHaveAttribute('data-jumps', 'hot hot hot hot');
	await expect(flown('rolling-reduced-crit')).toHaveAttribute('data-jumps', 'cold cold cold cold');

	// A lighter ship takes more jumps, and is remembered.
	await dialog.getByTestId('rolling-cold').fill('100');
	await dialog.getByTestId('rolling-hot').fill('200');
	await expect(flown('rolling-first')).toHaveAttribute('data-jumps', 'hot hot hot hot hot');
	await page.reload();
	await page.getByTestId('rolling-button').click();
	await expect(page.getByTestId('rolling-hot')).toHaveValue('200');

	// A frigate hole has no plan for it, and a tab takes the search back off.
	await page.getByTestId('rolling-find').fill('e004');
	await expect(page.getByTestId('rolling-plan')).toHaveCount(0);
	await expect(page.getByTestId('rolling-too-small')).toBeVisible();
	await expect(page.getByTestId('rolling-holes').locator('[data-fits=false]')).toContainText(
		'E004',
	);
	await page.getByRole('tab', { name: '3,300 kt' }).click();
	await expect(page.getByTestId('rolling-find')).toHaveValue('');
	await expect(page.getByTestId('rolling-plan')).toHaveCount(1);
});
