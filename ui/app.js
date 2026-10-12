"use strict";

const $ = (id) => document.getElementById(id);

const DEFAULT_PHASE = "Writing the next scene…";
const ILLUSTRATING_PHASE = "Creating illustration…";
const MAX_REFERENCE_SIDE = 1536;

// The launcher passes the API key in the URL fragment; keep it for this tab only.
const fragment = new URLSearchParams(location.hash.slice(1));
if (fragment.get("key")) {
  sessionStorage.setItem("hornbill-key", fragment.get("key"));
  history.replaceState(null, "", location.pathname + location.search);
}
const apiKey = sessionStorage.getItem("hornbill-key") || "";
const authHeader = { Authorization: "Bearer " + apiKey };

let selectedId = sessionStorage.getItem("hornbill-save") || "";
let current = null; // latest state response for the selected story
let busy = false; // a scene or illustration job is running
let lastJobId = 0;
let lastPhase = "";
let shownAttemptId = null;
let shownImageUrl = null;
let stories = [];

function createElement(tag, className, text) {
  const element = document.createElement(tag);
  if (className) element.className = className;
  if (text !== undefined) element.textContent = text;
  return element;
}

function notice(message) {
  $("notice").textContent = message || "";
  $("notice").hidden = !message;
  for (const old of document.querySelectorAll(".dialog-notice")) old.remove();
  // Open dialogs cover the page notice, so repeat it inside the dialog.
  const dialog = document.querySelector("dialog[open]");
  if (message && dialog) {
    const copy = createElement("p", "notice dialog-notice", message);
    copy.setAttribute("role", "alert");
    dialog.prepend(copy);
  }
}

async function api(path, body, method) {
  const response = await fetch("/api/" + path, {
    method: method || (body ? "POST" : "GET"),
    headers: body
      ? { ...authHeader, "Content-Type": "application/json" }
      : authHeader,
    body: body ? JSON.stringify(body) : undefined,
  });
  const text = await response.text();
  let data;
  try {
    data = JSON.parse(text);
  } catch {
    data = { error: text };
  }
  if (!response.ok) throw Error(data.error || "Request failed, try again");
  return data;
}

// Wraps an event handler so failures show up as a notice.
const reportErrors = (handler) => async (event) => {
  try {
    await handler(event);
  } catch (error) {
    notice(error.message);
  }
};

function showBusy(phase) {
  busy = true;
  $("phase").textContent = phase;
  $("elapsed").textContent = "Starting…";
  updateControls();
}

function updateControls() {
  for (const choice of document.querySelectorAll(".choice")) {
    choice.disabled = busy;
  }
  $("continue-button").disabled = busy || !current;
  $("illustrate-button").disabled = busy || !current?.state.current_scene;
  $("progress").hidden = !busy;
}

async function listStories() {
  stories = (await api("saves")).saves;
  $("saves").replaceChildren();
  for (const story of stories) {
    const button = createElement(
      "button",
      "save" + (story.id === selectedId ? " selected" : ""),
    );
    button.append(
      createElement("strong", "", story.title),
      createElement(
        "span",
        "",
        story.turn ? "Turn " + story.turn : "No turns yet",
      ),
    );
    button.onclick = reportErrors(() => selectStory(story.id));
    $("saves").append(button);
  }
  if (!stories.some((story) => story.id === selectedId)) {
    selectedId = stories[0]?.id || "";
  }
}

async function selectStory(id) {
  selectedId = id;
  sessionStorage.setItem("hornbill-save", id);
  notice("");
  await refresh();
  await listStories();
}

async function refresh() {
  if (!selectedId) {
    current = null;
    updateControls();
    return;
  }
  current = await api("state/" + encodeURIComponent(selectedId));
  render();
}

async function showImage(image) {
  if (image?.attempt_id === shownAttemptId) return;
  shownAttemptId = image?.attempt_id || null;
  $("scene-image").hidden = true;
  $("image-placeholder").hidden = false;
  if (shownImageUrl) {
    URL.revokeObjectURL(shownImageUrl);
    shownImageUrl = null;
  }
  if (!image) return;
  const requested = shownAttemptId;
  const response = await fetch("/api/image/" + image.attempt_id, {
    headers: authHeader,
  });
  if (!response.ok) return;
  const blob = await response.blob();
  if (shownAttemptId !== requested) return; // superseded while downloading
  shownImageUrl = URL.createObjectURL(blob);
  $("scene-image").src = shownImageUrl;
  $("scene-image").hidden = false;
  $("image-placeholder").hidden = true;
}

function renderDialogue(state, lines) {
  $("dialogue").replaceChildren();
  for (const line of lines) {
    const block = createElement("div", "speech");
    block.append(
      createElement(
        "strong",
        "",
        state.characters[line.speaker]?.name || line.speaker,
      ),
      createElement("p", "", line.text),
    );
    $("dialogue").append(block);
  }
}

function renderChoices(choices) {
  $("choices").replaceChildren();
  choices.forEach((text, index) => {
    const button = createElement("button", "choice");
    button.append(
      createElement("em", "", String(index + 1).padStart(2, "0")),
      createElement("span", "", text),
    );
    button.onclick = reportErrors(() => advance(text));
    $("choices").append(button);
  });
}

function imageLabel(image, turnNumber) {
  if (image.turn < turnNumber) return "Previous illustration";
  if (image.reason === "reused") return "Illustration reused";
  return "Illustration for this scene";
}

function render() {
  const { state, turn, image } = current;
  $("story-title").textContent =
    stories.find((story) => story.id === selectedId)?.title || "Story";
  $("chapter").textContent = state.turn ? "CURRENT SCENE" : "OPENING SCENE";
  $("turn-label").textContent = state.turn
    ? "Turn " + state.turn
    : "No turns yet";
  $("place").textContent = state.locations[state.world.location].name;
  $("scene-time").textContent = state.world.time + " · " + state.world.weather;
  $("narration").textContent = turn?.narration || state.summary;
  renderDialogue(state, turn?.dialogue || []);
  renderChoices(
    turn?.choices || [
      "I inspect the radio and ask Mira what she recognizes about the transmission.",
      "I look for a practical way into the observatory.",
    ],
  );
  $("image-label").hidden = !image;
  if (image) $("image-label").textContent = imageLabel(image, state.turn);
  void showImage(image).catch((error) => notice(error.message));
  updateControls();
}

async function advance(action) {
  if (!current || busy) return;
  notice("");
  await api("turn/" + selectedId, {
    action,
    expected_turn: current.state.turn,
  });
  $("action").value = "";
  showBusy(DEFAULT_PHASE);
}

function formatElapsed(startedMs) {
  const seconds = startedMs
    ? Math.max(0, Math.floor((Date.now() - startedMs) / 1000))
    : 0;
  return seconds >= 60
    ? Math.floor(seconds / 60) + "m " + (seconds % 60) + "s elapsed"
    : seconds + "s elapsed";
}

async function poll() {
  try {
    const job = await api("job");
    const wasBusy = busy;
    busy = job.running;
    $("phase").textContent = job.phase || DEFAULT_PHASE;
    $("elapsed").textContent = formatElapsed(job.started_ms);
    updateControls();
    const changed =
      lastJobId !== job.id || lastPhase !== job.phase || (wasBusy && !busy);
    if (job.id && changed) {
      lastJobId = job.id;
      lastPhase = job.phase;
      await refresh();
      await listStories();
      if (!busy && job.error) notice(job.error);
    }
  } catch (error) {
    notice(error.message);
  } finally {
    setTimeout(poll, busy ? 1000 : 4000);
  }
}

function openNewStoryDialog() {
  notice("");
  $("new-dialog").showModal();
}

function openSettingsDialog() {
  if (!current) {
    $("new-dialog").showModal();
    return;
  }
  const { model, image_policy, image } = current.preferences;
  $("model").value = model;
  $("image-policy").value = image_policy;
  $("image-size").value = image.width + "x" + image.height;
  $("image-seed").value = image.seed;
  $("settings-dialog").showModal();
}

function openJournalDialog() {
  if (!current) return;
  const { state } = current;
  $("summary").textContent = state.summary;
  $("inventory").replaceChildren(
    ...Object.entries(state.inventory).map(([name, count]) =>
      createElement("li", "", name.replaceAll("_", " ") + " × " + count),
    ),
  );
  $("threads").replaceChildren(
    ...Object.values(state.plot_threads)
      .filter((thread) => thread.status === "open")
      .map((thread) => createElement("li", "", thread.description)),
  );
  $("director-note").value = state.director_note || "";
  $("world-notes").value = state.lorebook.map((note) => note.text).join("\n\n");
  const subjects = [
    ...Object.values(state.characters),
    ...Object.values(state.locations),
  ];
  $("reference-entity").replaceChildren(
    ...subjects.map(
      (subject) =>
        new Option(
          subject.name +
            (state.references[subject.id] ? " · reference image attached" : ""),
          subject.id,
        ),
    ),
  );
  $("journal-dialog").showModal();
}

// Downscale to at most MAX_REFERENCE_SIDE px and re-encode as PNG.
async function encodeReferenceImage(file) {
  const bitmap = await createImageBitmap(file);
  const scale = Math.min(
    1,
    MAX_REFERENCE_SIDE / Math.max(bitmap.width, bitmap.height),
  );
  const canvas = document.createElement("canvas");
  canvas.width = Math.round(bitmap.width * scale);
  canvas.height = Math.round(bitmap.height * scale);
  canvas.getContext("2d").drawImage(bitmap, 0, 0, canvas.width, canvas.height);
  bitmap.close();
  return new Promise((resolve) => canvas.toBlob(resolve, "image/png"));
}

$("action-form").onsubmit = reportErrors(async (event) => {
  event.preventDefault();
  await advance($("action").value.trim());
});

$("action").onkeydown = (event) => {
  if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
    event.preventDefault();
    $("action-form").requestSubmit();
  }
};

$("illustrate-button").onclick = reportErrors(async () => {
  notice("");
  await api("illustrate/" + selectedId, { expected_turn: current.state.turn });
  showBusy(ILLUSTRATING_PHASE);
});

$("cancel-button").onclick = reportErrors(() => api("cancel", {}));

for (const button of document.querySelectorAll("[data-close]")) {
  button.onclick = () => $(button.dataset.close).close();
}

for (const id of ["new-button", "new-bottom", "new-top"]) {
  $(id).onclick = openNewStoryDialog;
}

$("new-form").onsubmit = reportErrors(async (event) => {
  event.preventDefault();
  const created = await api("saves", {
    title: $("new-title").value.trim(),
    premise: $("new-premise").value.trim() || null,
  });
  $("new-dialog").close();
  await listStories();
  await selectStory(created.id);
});

$("settings-button").onclick = openSettingsDialog;

$("settings-form").onsubmit = reportErrors(async (event) => {
  event.preventDefault();
  const [width, height] = $("image-size").value.split("x").map(Number);
  await api(
    "preferences/" + selectedId,
    {
      ...current.preferences,
      model: $("model").value,
      image_policy: $("image-policy").value,
      image: {
        ...current.preferences.image,
        width,
        height,
        seed: Number($("image-seed").value),
      },
    },
    "PUT",
  );
  $("settings-dialog").close();
  await refresh();
  notice("Settings saved.");
});

$("journal-button").onclick = openJournalDialog;

$("notes-form").onsubmit = reportErrors(async (event) => {
  event.preventDefault();
  const lorebook = $("world-notes")
    .value.split(/\n\s*\n/)
    .map((text) => text.trim())
    .filter(Boolean)
    .map((text, index) => ({ id: "note_" + index, text, tags: [] }));
  await api(
    "notes/" + selectedId,
    { director_note: $("director-note").value.trim() || null, lorebook },
    "PUT",
  );
  await refresh();
  $("journal-dialog").close();
  notice("Notes saved.");
});

$("reference-form").onsubmit = reportErrors(async (event) => {
  event.preventDefault();
  const file = $("reference-file").files[0];
  if (!file) return;
  const response = await fetch(
    "/api/reference/" + selectedId + "/" + $("reference-entity").value,
    {
      method: "POST",
      headers: authHeader,
      body: await encodeReferenceImage(file),
    },
  );
  if (!response.ok) {
    const data = await response.json();
    throw Error(data.error || "Could not save the reference image");
  }
  await refresh();
  $("journal-dialog").close();
  notice("Reference image attached. It will guide the next illustration.");
});

$("export-button").onclick = reportErrors(async () => {
  const data = await api("export/" + selectedId);
  const url = URL.createObjectURL(
    new Blob([JSON.stringify(data, null, 2)], { type: "application/json" }),
  );
  const link = document.createElement("a");
  link.href = url;
  link.download = selectedId + ".json";
  link.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
});

(async () => {
  try {
    await listStories();
    if (selectedId) await selectStory(selectedId);
    else $("new-dialog").showModal();
    await poll();
  } catch (error) {
    notice(error.message);
  }
})();
