import { expect, gotoApp, test } from './fixtures';
import { createMap } from './arrange';
import { E2E_CHARACTER_ID, E2E_CORPORATION_ID, withDb } from './db';

// Structures a member shares with a map: the card that shows them, the switch that
// decides whether it may, and the two alerts over them. What ESI says and what reaches
// Discord are the Rust tests' business; this is about what a member sees and controls.

const JITA = 30000142;
const ASTRAHUS = 35832;
const WEBHOOK = 'https://discord.com/api/webhooks/123456789/abcdefghijklmnop';

/**
 * What the watch would have stored for the e2e character: an attack five minutes ago on a
 * structure of their corporation, and that structure with two days of fuel.
 */
async function seedStructure(structureId: number) {
	await withDb(async (db) => {
		// The database outlives a run, and a corporation's list is the same on every map.
		await db.query('delete from corporation_structures where corporation_id = $1', [
			E2E_CORPORATION_ID,
		]);
		await db.query("delete from structure_events where dedup_key like 'e2e:%'");
		await db.query(
			`insert into corporation_structures
			     (structure_id, corporation_id, name, type_id, solar_system_id, state, fuel_expires)
			 values ($1, $2, 'Jita - E2E Home', $3, $4, 'shield_vulnerable', now() + interval '2 days')
			 on conflict (structure_id) do update set fuel_expires = excluded.fuel_expires`,
			[structureId, E2E_CORPORATION_ID, ASTRAHUS, JITA],
		);
		await db.query(
			`insert into corporation_structure_readers (character_id, corporation_id)
			 values ($1, $2)
			 on conflict (character_id) do update set read_at = now()`,
			[E2E_CHARACTER_ID, E2E_CORPORATION_ID],
		);
		const event = await db.query(
			`insert into structure_events
			     (corporation_id, kind, notification_type, structure_id, type_id, solar_system_id,
			      aggressor_corporation, shield, occurred_at, dedup_key)
			 values ($1, 'under_attack', 'StructureUnderAttack', $2, $3, $4, 'Evictors', 80,
			         now() - interval '5 minutes', $5)
			 on conflict (dedup_key) do update set occurred_at = excluded.occurred_at
			 returning id`,
			[E2E_CORPORATION_ID, structureId, ASTRAHUS, JITA, `e2e:${structureId}`],
		);
		await db.query(
			`insert into structure_event_receipts (structure_event_id, character_id)
			 values ($1, $2) on conflict do nothing`,
			[event.rows[0].id, E2E_CHARACTER_ID],
		);
	});
}

test('a map sees a member`s structures only while they share them', async ({ page, api }) => {
	const mapId = await createMap(api, 'E2E Structures');
	const structureId = 1_000_000_000_000 + mapId;
	await seedStructure(structureId);
	await api.post(`/api/maps/${mapId}/systems/add`, {
		data: { map_id: mapId, solar_system_id: JITA, x: 200, y: 200, alias: 'Market' },
	});

	await gotoApp(page, `/maps/${mapId}`);
	const card = page.getByTestId('structures-card');
	// Stored, received and readable, and still none of this map's business.
	await expect(card.getByTestId('structures-nobody')).toBeVisible();
	await expect(card.getByTestId('structure-event')).toHaveCount(0);

	await card.getByTestId('structures-sharing').click();
	await page.getByTestId('setting-structures_shared').click();

	const attack = card.getByTestId('structure-event').filter({ hasText: 'Jita - E2E Home' });
	await expect(attack).toContainText('is under attack');
	await expect(attack).toContainText('Jita');
	await expect(card.getByTestId('structures-attacks')).toBeVisible();
	const fuel = card.getByTestId('structure-fuel').filter({ hasText: 'Jita - E2E Home' });
	await expect(fuel).toHaveAttribute('data-tone', 'critical');
	await expect(fuel).toContainText('1d');

	// Done with the sharing popover; it holds the keyboard while it is open.
	await page.keyboard.press('Escape');
	await expect(page.getByTestId('setting-structures_shared')).toHaveCount(0);

	// The row speaks the map's language: its alias for the system, and its region.
	await expect(attack.getByTestId('structure-system')).toContainText('Market');
	await expect(attack).toContainText('The Forge');

	// Search looks through what a row shows, the alias included.
	const search = card.getByTestId('structures-search');
	await search.fill('market home');
	await expect(attack).toBeVisible();
	await search.fill('nowhere');
	await expect(card.getByTestId('structures-no-match')).toBeVisible();
	await search.clear();
	await expect(search).toHaveValue('');
	await expect(attack).toBeVisible();

	// The filters take a kind of news away, and the rest stays.
	await card.getByTestId('structures-filter-attack').click();
	await expect(card.getByTestId('structure-event')).toHaveCount(0);
	await expect(fuel).toBeVisible();
	await card.getByTestId('structures-filter-fuel').click();
	await expect(card.getByTestId('structures-no-match')).toBeVisible();
	await card.getByTestId('structures-filter-attack').click();
	await card.getByTestId('structures-filter-fuel').click();
	await expect(attack).toBeVisible();

	// Off again, and it is gone again.
	await card.getByTestId('structures-sharing').click();
	await page.getByTestId('setting-structures_shared').click();
	await expect(card.getByTestId('structures-nobody')).toBeVisible();
});

test('a fuel alert asks for days, not jumps or a system', async ({ page, api }) => {
	const mapId = await createMap(api, 'E2E Structure Alerts');
	await gotoApp(page, `/maps/${mapId}/settings/alerts`);
	await page.getByTestId('destination-name').fill('Ops channel');
	await page.getByTestId('destination-url').fill(WEBHOOK);
	await page.getByTestId('destination-add').click();
	await expect(page.getByTestId('destination-row')).toHaveCount(1);

	await page.getByTestId('alert-new').click();
	await page.getByTestId('alert-name').fill('Fuel');
	await page.getByTestId('alert-kind').click();
	await page.getByRole('option', { name: 'Structure fuel' }).click();
	await expect(page.getByTestId('alert-jumps')).toHaveCount(0);
	await page.getByTestId('alert-fuel-days').fill('7');
	await page.getByTestId('alert-destination').click();
	await page.getByRole('option', { name: 'Ops channel' }).click();
	await page.getByTestId('alert-save').click();

	const row = page.getByTestId('alert-row');
	await expect(row).toContainText('Shared structures: 7 days of fuel left');

	// An attack alert has nothing to set beyond where it goes.
	await page.getByTestId('alert-new').click();
	await page.getByTestId('alert-name').fill('Defence');
	await page.getByTestId('alert-kind').click();
	await page.getByRole('option', { name: 'Structure attacks and changes' }).click();
	await expect(page.getByTestId('alert-fuel-days')).toHaveCount(0);
	// Which events it speaks about is the alert's own choice: the bad news is ticked, the
	// routine is not, and either can be changed.
	await expect(page.getByTestId('alert-event-under_attack')).toBeChecked();
	await expect(page.getByTestId('alert-event-online')).not.toBeChecked();
	await page.getByTestId('alert-event-under_attack').uncheck();
	await page.getByTestId('alert-event-online').check();
	await page.getByTestId('alert-destination').click();
	await page.getByRole('option', { name: 'Ops channel' }).click();
	await page.getByTestId('alert-save').click();
	await expect(page.getByTestId('alert-row')).toHaveCount(2);
	const defence = page.getByTestId('alert-row').filter({ hasText: 'Defence' });
	await expect(defence).toContainText('Shared structures: lost shields');
	await expect(defence).toContainText('came online');
	await expect(defence).not.toContainText('under attack');

	// And it is still the choice when the alert is opened again.
	await defence.getByTestId('alert-edit').click();
	await expect(page.getByTestId('alert-event-under_attack')).not.toBeChecked();
	await expect(page.getByTestId('alert-event-online')).toBeChecked();
});
