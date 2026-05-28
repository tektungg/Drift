import { test } from "node:test";
import assert from "node:assert/strict";
import { matchesSearch, filterTorrents, compareBy, sortTorrents } from "./list-ops.js";

const A = { infohash: "a", name: "Ubuntu 24.04", state_label: "downloading", downloaded: 50, total: 100, down_bps: 10, total_size: 100, added_at: 3 };
const B = { infohash: "b", name: "Debian 12",    state_label: "seeding",     downloaded: 100, total: 100, down_bps: 0,  total_size: 200, added_at: 1 };
const C = { infohash: "c", name: "ubuntu-server", state_label: "downloading", downloaded: 10,  total: 100, down_bps: 99, total_size: 50,  added_at: 2 };

test("matchesSearch is case-insensitive substring", () => {
  assert.equal(matchesSearch(A, "ubuntu"), true);
  assert.equal(matchesSearch(C, "UBUNTU"), true);
  assert.equal(matchesSearch(B, "ubuntu"), false);
  assert.equal(matchesSearch(A, ""), true); // empty query matches all
});

test("filterTorrents composes state filter AND search", () => {
  const out = filterTorrents([A, B, C], "downloading", "ubuntu");
  assert.deepEqual(out.map(t => t.infohash), ["a", "c"]);
  assert.deepEqual(filterTorrents([A, B, C], "all", "").map(t => t.infohash), ["a", "b", "c"]);
  assert.deepEqual(filterTorrents([A, B, C], "seeding", "").map(t => t.infohash), ["b"]);
});

test("compareBy progress ascending", () => {
  // progress = downloaded/total: A=0.5, C=0.1 -> C before A ascending
  const cmp = compareBy("progress", "asc");
  assert.equal(cmp(A, C) > 0, true);
});

test("sortTorrents by added desc is default-friendly", () => {
  assert.deepEqual(sortTorrents([B, C, A], "added", "desc").map(t => t.infohash), ["a", "c", "b"]);
});

test("sortTorrents by name asc", () => {
  assert.deepEqual(sortTorrents([A, B, C], "name", "asc").map(t => t.infohash), ["b", "a", "c"]);
});

test("sortTorrents by speed desc", () => {
  assert.deepEqual(sortTorrents([A, B, C], "speed", "desc").map(t => t.infohash), ["c", "a", "b"]);
});

test("sortTorrents by size desc uses total_size", () => {
  assert.deepEqual(sortTorrents([A, B, C], "size", "desc").map(t => t.infohash), ["b", "a", "c"]);
});

test("sortTorrents does not mutate input", () => {
  const arr = [A, B, C];
  sortTorrents(arr, "name", "asc");
  assert.deepEqual(arr.map(t => t.infohash), ["a", "b", "c"]);
});
