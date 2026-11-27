/* LabelForge shim around the Zint barcode library.
 * Free software under the GNU AGPL v3 or later (this file); Zint itself is BSD-3.
 *
 * Encodes one symbol and writes its vector output as an SVG fragment in
 * Zint's vector units, so Rust never needs to know Zint's struct layout.
 */
#include <stdio.h>
#include <string.h>
#include "zint/zint.h"

typedef struct {
    char *buf;
    int cap;
    int len;
    int overflow;
} sb;

static void put(sb *s, const char *fmt, double a, double b, double c, double d) {
    if (s->overflow) return;
    int n = snprintf(s->buf + s->len, (size_t)(s->cap - s->len), fmt, a, b, c, d);
    if (n < 0 || n >= s->cap - s->len) { s->overflow = 1; return; }
    s->len += n;
}

static void puts_raw(sb *s, const char *t) {
    if (s->overflow) return;
    int n = (int)strlen(t);
    if (n >= s->cap - s->len) { s->overflow = 1; return; }
    memcpy(s->buf + s->len, t, (size_t)n);
    s->len += n;
    s->buf[s->len] = 0;
}

static void put_escaped(sb *s, const unsigned char *t) {
    char one[2] = {0, 0};
    for (; *t; t++) {
        switch (*t) {
        case '&': puts_raw(s, "&amp;"); break;
        case '<': puts_raw(s, "&lt;"); break;
        case '>': puts_raw(s, "&gt;"); break;
        case '"': puts_raw(s, "&quot;"); break;
        default: one[0] = (char)*t; puts_raw(s, one);
        }
    }
}

/* Returns 0 on success (warnings allowed), else Zint's error code, with the
 * message in err. On success out holds "<path .../>..." elements using
 * fill="currentColor", and *vw/*vh the vector size, *modules the symbol
 * width in modules, *rows its row count and *hx the bar height used
 * (in X-dimensions). */
int lf_zint_svg(int symbology, const unsigned char *data, int len, int input_mode,
                int option_1, int option_2, int option_3, float height_x, int show_hrt, int quiet,
                const char *primary, char *out, int outcap, float *vw, float *vh,
                int *modules, int *rows, float *hx, char *err, int errcap) {
    struct zint_symbol *sym = ZBarcode_Create();
    if (!sym) { snprintf(err, (size_t)errcap, "out of memory"); return 99; }
    sym->symbology = symbology;
    sym->input_mode = input_mode;
    sym->option_1 = option_1;
    sym->option_2 = option_2;
    sym->option_3 = option_3;
    sym->show_hrt = show_hrt;
    sym->output_options = quiet ? BARCODE_QUIET_ZONES : BARCODE_NO_QUIET_ZONES;
    sym->whitespace_width = 0;
    sym->whitespace_height = 0;
    sym->scale = 0.5f; /* 1 vector unit = 1 module */
    if (height_x > 0) sym->height = height_x;
    if (primary && primary[0]) {
        strncpy(sym->primary, primary, sizeof(sym->primary) - 1);
    }
    int rc = ZBarcode_Encode_and_Buffer_Vector(sym, data, len, 0);
    if (rc >= ZINT_ERROR) {
        snprintf(err, (size_t)errcap, "%s", sym->errtxt);
        ZBarcode_Delete(sym);
        return rc;
    }
    err[0] = 0;
    if (rc != 0) snprintf(err, (size_t)errcap, "%s", sym->errtxt); /* warning */
    struct zint_vector *v = sym->vector;
    sb s = {out, outcap, 0, 0};
    out[0] = 0;
    *vw = v->width;
    *vh = v->height;
    *modules = sym->width;
    *rows = sym->rows;
    *hx = sym->height;

    puts_raw(&s, "<path fill=\"currentColor\" shape-rendering=\"crispEdges\" d=\"");
    for (struct zint_vector_rect *r = v->rectangles; r; r = r->next) {
        if (r->colour == 8) continue; /* white */
        put(&s, "M%.4f %.4fh%.4fv%.4f", r->x, r->y, r->width, r->height);
        put(&s, "h%.4fz", -r->width, 0, 0, 0);
    }
    puts_raw(&s, "\"/>");
    if (v->hexagons) {
        puts_raw(&s, "<path fill=\"currentColor\" d=\"");
        for (struct zint_vector_hexagon *h = v->hexagons; h; h = h->next) {
            /* Short diameter d is the inscribed circle; radius to vertex = d/sqrt(3). */
            double R = h->diameter / 1.7320508075688772;
            double hw = h->diameter / 2.0;
            if (h->rotation == 0 || h->rotation == 180) {
                put(&s, "M%.4f %.4f", h->x, h->y - R, 0, 0);
                put(&s, "L%.4f %.4fL%.4f %.4f", h->x + hw, h->y - R / 2, h->x + hw, h->y + R / 2);
                put(&s, "L%.4f %.4fL%.4f %.4f", h->x, h->y + R, h->x - hw, h->y + R / 2);
                put(&s, "L%.4f %.4fz", h->x - hw, h->y - R / 2, 0, 0);
            } else {
                put(&s, "M%.4f %.4f", h->x - R, h->y, 0, 0);
                put(&s, "L%.4f %.4fL%.4f %.4f", h->x - R / 2, h->y - hw, h->x + R / 2, h->y - hw);
                put(&s, "L%.4f %.4fL%.4f %.4f", h->x + R, h->y, h->x + R / 2, h->y + hw);
                put(&s, "L%.4f %.4fz", h->x - R / 2, h->y + hw, 0, 0);
            }
        }
        puts_raw(&s, "\"/>");
    }
    for (struct zint_vector_circle *c = v->circles; c; c = c->next) {
        if (c->width > 0) {
            put(&s, "<circle cx=\"%.4f\" cy=\"%.4f\" r=\"%.4f\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"%.4f\"/>",
                c->x, c->y, c->diameter / 2.0, c->width);
        } else {
            put(&s, "<circle cx=\"%.4f\" cy=\"%.4f\" r=\"%.4f\" fill=\"", c->x, c->y, c->diameter / 2.0, 0);
            puts_raw(&s, c->colour ? "#ffffff\"/>" : "currentColor\"/>");
        }
    }
    for (struct zint_vector_string *t = v->strings; t; t = t->next) {
        const char *anchor = t->halign == 1 ? "start" : (t->halign == 2 ? "end" : "middle");
        put(&s, "<text x=\"%.4f\" y=\"%.4f\" font-size=\"%.4f\"", t->x, t->y, t->fsize, 0);
        if (t->rotation) put(&s, " transform=\"rotate(%.0f %.4f %.4f)\"", t->rotation, t->x, t->y, 0);
        puts_raw(&s, " font-family=\"OCR-B, Helvetica, Arial, sans-serif\" fill=\"currentColor\" text-anchor=\"");
        puts_raw(&s, anchor);
        puts_raw(&s, "\">");
        put_escaped(&s, t->text);
        puts_raw(&s, "</text>");
    }
    ZBarcode_Delete(sym);
    if (s.overflow) { snprintf(err, (size_t)errcap, "barcode too large"); return 98; }
    return 0;
}
