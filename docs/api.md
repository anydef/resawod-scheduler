# Nubapp API Reference

The RESAWOD booking system runs on the Nubapp platform (`sport.nubapp.com`). This document describes all API endpoints used by the scheduler.

## Base URL

```
https://sport.nubapp.com/web
```

## Authentication Flow

Authentication is session-based (PHP sessions). The flow is:

1. **Init session** — GET `cookieChecker.php` to establish a server-side PHP session
2. **Login** — POST credentials to `checkUser.php` to authenticate the session
3. All subsequent requests reuse the authenticated session via cookies

All requests include spoofed browser headers (Chrome on Android) to mimic a real browser.

---

## Endpoints

### 1. Session Initialization

```
GET /web/cookieChecker.php
```

**Query Parameters:**

| Parameter        | Type   | Example      | Description                |
|------------------|--------|--------------|----------------------------|
| `id_application` | string | `"36307036"` | Gym/box identifier         |
| `isIframe`       | string | `"false"`    | Always `"false"`           |

**Required Headers:**

| Header    | Value                              |
|-----------|------------------------------------|
| `Cookie`  | `applicationId={id_application}`   |
| `Referer` | `https://sport.nubapp.com/web/setApplication.php?id_application={id_application}` |

**Purpose:** Establishes a PHP session. The response sets session cookies that are used for all subsequent requests.

---

### 2. User Login

```
POST /web/ajax/users/checkUser.php
```

**Content-Type:** `application/x-www-form-urlencoded`

**Body Parameters:**

| Parameter  | Type   | Description          |
|------------|--------|----------------------|
| `username` | string | User email address   |
| `password` | string | User password        |

**Response:** JSON object containing user account data, including:
- `resasocialAccountData.boundApplicationData.id_application`

---

### 3. Fetch Available Slots

```
GET /web/ajax/activities/getActivitiesCalendar.php
```

**Query Parameters:**

| Parameter              | Type   | Example    | Description                                  |
|------------------------|--------|------------|----------------------------------------------|
| `id_category_activity` | string | `"2179"`   | Activity category (e.g., CrossFit WOD)       |
| `offset`               | string | `"-120"`   | Timezone offset in minutes (UTC+2 = `-120`)  |
| `start`                | string | UNIX ts    | Start of search window (00:00 of target day) |
| `end`                  | string | UNIX ts    | End of search window (22:00 of target day)   |
| `_`                    | string | UNIX ts    | Cache-busting parameter (current timestamp)  |

**Response:** JSON array of slot objects:

```json
[
  {
    "start": "2024-01-15 18:30:00",
    "end": "2024-01-15 19:30:00",
    "id_activity_calendar": "12345"
  }
]
```

---

### 4. Book a Slot

```
POST /web/ajax/bookings/bookBookings.php
```

**Content-Type:** `application/x-www-form-urlencoded`

**Body Parameters:**

| Parameter                                       | Type   | Value   | Description        |
|-------------------------------------------------|--------|---------|--------------------|
| `items[activities][0][id_activity_calendar]`    | string | slot ID | The slot to book   |
| `items[activities][0][unit_price]`              | string | `"0"`   | Price (usually 0)  |
| `items[activities][0][n_guests]`                | string | `"0"`   | Number of guests   |
| `items[activities][0][id_resource]`             | string | `"false"` | Resource ID      |
| `discount_code`                                 | string | `"false"` | Discount code    |
| `form`                                          | string | `""`    | Form data          |
| `formIntoNotes`                                 | string | `""`    | Notes              |

**Response:** JSON confirmation of the booking.

---

### 5. List Activity Categories

```
GET /web/ajax/activities/getCategoriesActivities.php
```

**Query Parameters:** None (uses the authenticated session).

**Purpose:** Returns all activity categories available at the gym. Used by the `discover` command to help users find their `category_activity_id`.

**Response:** JSON array of category objects (structure varies by gym), typically containing:
- `id_category_activity` or `id` — The category ID to use in config
- `name` or `title` — Human-readable category name

---

## Gym-Specific Configuration

Each gym has unique identifiers that must be configured:

| Value                  | Description                              | How to Find                        |
|------------------------|------------------------------------------|------------------------------------|
| `application_id`       | Gym identifier on Nubapp                 | From gym's RESAWOD booking URL     |
| `category_activity_id` | Activity type (e.g., CrossFit, Open Gym) | Use the `discover` command, or network inspector |

The defaults in this project are:
- `application_id`: `36307036`
- `category_activity_id`: `2179`

Run `resawod-scheduler discover` to find these values (see [usage.md](usage.md)).

Override in the `book` subcommand with `--application-id` and `--category-activity-id` flags.
