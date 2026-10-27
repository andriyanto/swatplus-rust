## Bab 12  Routing: unit routing, lanskap, dan saluran

Bab ini menjelaskan bagian engine yang membawa air dan muatannya dari HRU ke luar DAS: jaringan objek dan loop perintah (`command`), routing lanskap dari satu HRU ke HRU lain, unit routing (*routing unit*, RU) beserta *delivery ratio*-nya, hidrograf sub-harian (unit hidrograf, `tsin`), serta saluran SWAT-DEG (`sd_channel`): kurva rating, routing tampungan (Muskingum atau *variable storage coefficient*), sedimen dataran banjir dan erosi tebing, rembesan dan penguapan, kualitas air QUAL2E, dan suhu air sungai. Dua file Rust dicakup seluruhnya: `crates/swat-engine/src/route.rs` (struct `Network`) dan `crates/swat-engine/src/channel.rs` (struct `Channels`), ditambah ringkasan `docs/routing.md`. Fungsi akuifer di `route.rs` (`aqu_1d_control`, `AquDay::columns`, `AquDay::columns_as`, `Network::aquifers`, dan bagian `aqu_initial` di `Network::new`) hanya disebut di sini; uraiannya ada di Bab 14. Reservoir dan objek *recall* (point source) diimplementasikan di `res.rs`; bab ini hanya menjelaskan bagaimana loop perintah memanggil keduanya.

Posisi dalam alur simulasi harian: setelah cuaca dibangkitkan, HRU dijalankan dalam urutan perintah SWAT+ (`Network::hru_order`); HRU yang menerima air HRU lain mendapat *runon* lewat `Network::hru_runon` sebelum evapotranspirasinya sendiri, lalu hidrograf HRU dicatat ke jaringan dengan `Network::set_hru`. Setelah semua HRU selesai, `Network::day` menjalankan objek-objek selebihnya (unit routing, akuifer, saluran SWAT-DEG, reservoir, recall, outlet) dalam urutan perintah, masing-masing menerima jumlah hidrograf dari objek pengirimnya. Padanan Fortran utama: `hyd_connect.f90`, `hyd_read_connect.f90`, `command.f90`, `ru_control.f90`, `dr_ru.f90`, `time_conc_init.f90`, `unit_hyd.f90`, `unit_hyd_ru_hru.f90`, `flow_hyd_ru_hru.f90`, `hru_hyds.f90`, `rls_routesurf.f90`, `rls_routesoil.f90`, `sd_hydsed_init.f90`, `sd_rating_curve.f90`, `sd_channel_control3.f90`, `sd_channel_sediment3.f90`, `ch_rtmusk.f90`, `ch_watqual4.f90`, `ch_temp.f90`, `rcurv_interp_flo.f90`, `qman.f90`, `theta.f90`, `wq_semianalyt.f90`, `wq_k2m.f90`, `ch_read_nut.f90`.

Semua operasi hidrograf memakai tipe `Hyd` (Bab tentang `hyd.rs`): `plus` (`hydout_add`, jumlah massa; suhu dirata-rata tertimbang debit), `minus`, `scale(k, h)` (`hydout_mult_const`), `times` (`hydout_mult`, perkalian komponen demi komponen), `over` (pembagian dengan konstanta) dan `add_const`. Satuan di hidrograf objek: debit `flo` dalam m3 per hari, massa (sedimen dalam ton, nutrien dalam kg), suhu dalam °C.

### 12.1  Struktur jaringan objek dan inisialisasinya (`route.rs`)

SWAT+ menyimpan semua objek ruang dalam satu larik `ob` (`sp_ob1` memberi indeks awal tiap jenis). Port mempertahankan urutan jenis yang sama lewat konstanta `KINDS`, karena urutan ini menentukan urutan perintah dan dengan demikian urutan penjumlahan bilangan f32 (penting untuk hasil bit-for-bit).

| Jenis (`ObjectKind`) | Kode | Jumlah hidrograf (`hd_tot`) | Didukung port |
| --- | --- | --- | --- |
| `Hru` | hru | 5 (tot, rhg, sur, lat, til) | ya |
| `HruLte` | hlt | 5 | tidak (error) |
| `RoutingUnit` | ru | 5 | ya |
| `Gwflow` | gwflow | 1 | tidak |
| `Aquifer` | aqu | 2 (aliran balik, rembesan dalam) | ya (Bab 14) |
| `Channel` | cha | 3 | tidak |
| `Reservoir` | res | 2 | ya |
| `Recall` | rec | 1 | ya |
| `ExportCoefficient` | exc | 2 | tidak |
| `DeliveryRatio` | dr | 2 | tidak |
| `Outlet` | out | 1 | ya (meneruskan inflow) |
| `ChannelDeg` | sdc | 3 | ya |

Struct privat dan publik di `route.rs`:

| Struct | Isi utama |
| --- | --- |
| `Out` | satu baris keluaran objek (`obj_out`, `obtyp_out`, `ihtyp_out`, `frac_out`): objek penerima `to` (indeks larik, `None` bila nomor objek 0), kode jenis penerima, nomor hidrograf `ihtyp` (1 tot, 2 rhg, 3 sur, 4 lat, 5 til) dan fraksi `frac` (-) |
| `In` | sisi penerima dari `Out`: objek pengirim `from`, `ihtyp`, `frac` |
| `Ob` | satu objek: jenis, nama, `gis_id`, luas `area_ha` (ha), stasiun cuaca `wst` (1-based), pointer data `props`, daftar `outs`/`ins`, hidrograf `hd` (m3, ton, kg), jumlah inflow hari ini `hin`, RU tempatnya menjadi elemen `subs` (`obj_subs`), jumlah elemen RU `dfn_tot`; untuk langkah sub-harian: `hyd_flo` (hidrograf debit per langkah untuk `day_max` hari, datar), `uh` (unit hidrograf), `day_cur`, `day_max`, `tsin` (inflow per langkah) |
| `RuElem` | elemen RU: indeks objek, apakah HRU, fraksi `frac` dan *delivery ratio* `dr` (sebuah `Hyd` faktor pengali) |
| `Ru` | RU: indeks objek, luas `da_km2` (km2), elemen, waktu konsentrasi `tc` (jam, `ru_tc`) |
| `AquDay`, `Aqu` | keadaan akuifer harian (Bab 14) |
| `Network` | seluruh jaringan: `obs`, urutan perintah `order` (tanpa HRU) dan `hru_order`, `rus`, peta `ru_of`/`aqu_of`/`chan_of`/`res_of`/`rec_of` dari indeks objek ke indeks per jenis, `ru_d` (keluaran harian RU), `fixes`, `channels: Option<Channels>`, `chan_in` (asal inflow tiap saluran untuk suhu air), `channel_skipped` (alasan saluran tidak dirouting), `reservoirs`, `recalls`, `step` (langkah per hari), `ga` (Green-Ampt), `dtm` (menit per langkah), `tconc` (waktu konsentrasi HRU, jam), `hhsurfq` (limpasan HRU per langkah, mm) |
| `NetDay` | masukan harian loop perintah: cuaca per stasiun `weat`, panjang hari `daylth` (jam), pengukur suhu `tgage`, neraca air HRU `hru_wb` (salju leleh, limpasan permukaan, aliran lateral, *water yield*; mm), `yrs`, `jday`, `yrc`, `mo`, `day_mo`, `print` |
| `ChannelDay` | alias tuple untuk baris `channel_sd_day`: nomor, GIS id, nama, `ChWat`, tampungan, inflow, outflow, suhu air |

#### 12.1.1  `Network::new`

- **Lokasi:** `crates/swat-engine/src/route.rs`, `pub fn new(project: &Project, stations: &[String], tconc: &[f32], ovn_km: &[(f32, f32)], fixes: bool) -> Result<Option<Network>, String>`
- **Padanan SWAT+:** `hyd_connect.f90` (larik objek, keluaran/masukan, urutan perintah `cmd_next`), `hyd_read_connect.f90` (`day_max`, alokasi `hyd_flo`, `tsin`, `uh`), `ru_read_elements.f90`, `time_conc_init.f90` (waktu konsentrasi RU), `dr_ru.f90` (*delivery ratio*), `aqu_initial.f90` dan `cal_conditions.f90` untuk akuifer (Bab 14), `sd_channel_read.f90`/`sd_hydsed_init.f90` (lewat `Channels::new`), `res_read`/`res_initial`/`recall_read` (lewat `res.rs`), `unit_hyd_ru_hru.f90`.
- **Tujuan:** membangun seluruh jaringan objek di luar HRU satu kali sebelum simulasi: siapa mengirim apa ke siapa, urutan eksekusi, parameter RU, akuifer, saluran, reservoir, recall, dan unit hidrograf.
- **Masukan dan keluaran:** `project` (tabel input yang sudah dibaca), nama stasiun cuaca, `tconc` per HRU (jam), `ovn_km` per HRU (n Manning lahan, luas km2). Mengembalikan `Ok(None)` bila proyek hanya berisi HRU (tidak ada jaringan), `Err(teks)` yang menamai fitur yang belum diporting, atau `Ok(Some(Network))`.
- **Cara kerja:**
  1. Untuk setiap jenis di `KINDS` secara berurutan, setiap objek dari file `.con` dimasukkan ke `obs` dengan `hd_tot` hidrograf kosong; stasiun cuaca dicari dengan `fortran::search` (tidak ketemu → 1). Indeks awal tiap jenis disimpan (`first`, padanan `sp_ob1`).
  2. Bila banyaknya objek sama dengan banyaknya HRU, kembalikan `None`. Jenis selain HRU, RU, akuifer, saluran SWAT-DEG, outlet, reservoir, recall, dan akuifer 2-D, menghasilkan `Err`.
  3. **Keluaran.** Untuk tiap baris keluaran: kode jenis tujuan diterjemahkan ke `ObjectKind`; indeks tujuan = `first[jenis] + obj_id − 1` (atau `None` bila `obj_id` = 0); kode hidrograf `tot/rhg/sur/lat/til` → 1..5. Air yang dikirim ke RU, atau dari objek non-HRU ke HRU, ditolak (`Err`), sesuai `docs/routing.md`. Setiap keluaran dengan tujuan juga dicatat sebagai `In` pada objek penerima, dalam urutan objek pengirim (urutan ini yang dipakai saat menjumlahkan inflow).
  4. **Unit routing.** Untuk RU ke-`iru`: luas `da_km2 = area_ha/100`. Setiap elemen (dari `rout_unit.def` dan `rout_unit.ele`) harus bertipe `hru` (selain itu `Err`); RU dicatat di `subs` HRU tersebut, dan dijumlahkan `ru_n += ovn · km2`. `dfn_tot` = banyaknya elemen.
  5. **Waktu konsentrasi RU** (`time_conc_init`). Manning lahan rata-rata tertimbang luas, lalu waktu aliran permukaan dan aliran alur, dengan kemiringan `slope` dan panjang lereng `slope_len` dari `topography.hyd` (default 0,02 dan 50 m) dan panjang alur `length` dari `field.fld` (default 500 m):

```latex
\bar n = \frac{\sum_e n_{ov,e} A_e}{A_{RU}}, \qquad t_{ov} = \frac{0.0556\,(L_{slp}\,\bar n)^{0.6}}{(S+0.001)^{0.3}}
```

```latex
S_{ch} = 0.5\,(S+0.001), \quad L_{ch} = \frac{length}{1000}, \quad t_{ch} = \frac{0.62\,L_{ch}\,\bar n^{0.75}}{A_{RU}^{0.125}\,S_{ch}^{0.375}}, \quad t_{c,RU} = t_{ov} + t_{ch}
```

dengan A dalam km2, L\_ch dalam km, waktu dalam jam. Perhatikan bahwa n Manning alur diambil sama dengan n lahan rata-rata (`ch_n = ru_n`), persis seperti SWAT+.

6. **Delivery ratio** (`dr_ru`). Nama `dr` elemen `calc` atau `0` (yang ditulis SWAT+ Editor): rasio dihitung dari waktu konsentrasi HRU terhadap RU,

```latex
DR = \min\left(1,\ \left(\frac{t_{c,HRU}}{t_{c,RU}}\right)^{0.5}\right)
```

Rasio ini disimpan sebagai `Hyd` yang semua komponennya `DR` kecuali `flo = 1` (air tidak dikurangi, hanya muatan). Nama `full` atau `1`: semua komponen 1. Nama lain (tabel `delratio.del`) belum diporting → `Err`. Pangkat memakai `rpow` (padanan `**` Fortran).

7. **Akuifer** (`aqu_initial`) dan kalibrasinya: lihat Bab 14.
8. **Urutan perintah** (`cmd_next` di `hyd_connect`). Dilakukan beberapa lintasan atas larik objek; pada setiap lintasan, objek yang belum jalan, yang seluruh pengirimnya sudah jalan (`rcv_sum == rcv_tot`) dan, untuk RU, seluruh elemennya sudah jalan (`dfn_sum == dfn_tot`), ditandai jalan; penghitung penerima dan RU-nya dinaikkan *di dalam lintasan yang sama*, sehingga objek berikutnya dalam larik bisa langsung jalan pada lintasan itu. HRU masuk ke `hru_order`, objek lain ke `order`. Lebih dari 5000 lintasan → `Err` ("infinite loop"), sama dengan batas SWAT+.
9. **Saluran SWAT-DEG.** `Channels::new` dipanggil dengan pointer data (`props`) tiap saluran. Bila gagal, alasannya disimpan di `channel_skipped` dan saluran bertindak sebagai objek penerus (outflow = inflow). Untuk setiap saluran juga dicatat, per inflow, apakah ia berasal dari saluran SWAT-DEG dan RU mana saja yang menjadi inflow (untuk `ch_temp`).
10. **Reservoir dan recall**: dibuat lewat `res::Reservoir::new` dan `res::Recall::new`; reservoir dengan `lapse` = 1 ditolak.
11. **Hidrograf sub-harian** (`hyd_read_connect`): `day_max` = 2 untuk HRU, HRU-lte, RU, saluran, outlet, dan saluran SWAT-DEG (dua hari, `day_lag_mx`), 1 untuk lainnya; `hyd_flo` berukuran `day_max · step`, `tsin` berukuran `step`.
12. **Unit hidrograf** (`unit_hyd_ru_hru`): bila `step > 1` dan bukan Green-Ampt (`gampt` ≠ 1), setiap HRU mendapat `unit_hyd(tconc_HRU)` dan setiap RU `unit_hyd(tc_RU)`, dengan parameter `tb_adj` dan `uhalpha` dari `parameters.bsn` dan bentuk `uhyd` dari `codes.bsn`.

- **Kebiasaan SWAT+ / catatan port:** urutan lintasan `cmd_next` dipertahankan persis karena menentukan urutan penjumlahan inflow. *Delivery ratio* tidak dikenakan pada hidrograf recharge (lihat `ru_control`). Saluran dengan panjang ≤ 1 mm (`chl ≤ 1e-3` km, saluran penerus SWAT+) membuat `Channels::new` gagal sehingga seluruh saluran menjadi penerus; ini disengaja (lihat 12.6). Unit hidrograf untuk inflow HRU/RU dengan fraksi < 1 (`hin_uh` di `unit_hyd_ru_hru`) tidak dibuat, karena tidak dipakai dalam alur yang diporting.

### 12.2  Hidrograf sub-harian: unit hidrograf, pergeseran, dan `tsin` (`route.rs`)

Bila `time.sim` memakai lebih dari satu langkah per hari (`step > 1`, panjang langkah `dtm = 1440/step` menit), setiap objek menyimpan hidrograf debit per langkah `hyd_flo` untuk `day_max` hari (dua hari untuk HRU, RU, saluran, outlet; satu untuk lainnya), disusun datar: elemen `(hari − 1)·step + langkah`. Dua jalur dibedakan: dengan Green-Ampt (`gampt` = 1) limpasan per langkah dari HRU (`hhsurfq`, mm) digeser sejauh waktu konsentrasi; dengan bilangan kurva (`gampt` = 0) limpasan harian disebar dengan unit hidrograf. Lihat juga `docs/subdaily.md`. Struct pendukung: `UhParams` berisi `step`, `dtm` (menit), `tb_adj` (jam, `adj_uhyd`), `uhyd` (0 segitiga, 1 gamma) dan `uhalpha` (alfa gamma).

#### 12.2.1  `unit_hyd`

- **Lokasi:** `crates/swat-engine/src/route.rs`, `fn unit_hyd(tc: f32, p: &UhParams) -> Vec<f32>` (panjang `2·step`).
- **Padanan SWAT+:** `unit_hyd.f90`, dipanggil dari `unit_hyd_ru_hru.f90`.
- **Tujuan:** membuat unit hidrograf tak berdimensi dua hari (per langkah) untuk waktu konsentrasi `tc` (jam), yang jumlahnya 1.
- **Masukan dan keluaran:** `tc` (jam), `UhParams`; keluaran fraksi limpasan per langkah untuk hari ini dan besok.
- **Cara kerja:**

1. Waktu dasar dan puncak:

```latex
t_b = \min(48,\ 0.5 + 0.6\,t_c + t_{b,adj}), \qquad t_p = 0.375\,t_b
```

2. Banyaknya langkah dalam waktu dasar `ts_base = max(1, int(t_b / (dtm/60)))`, banyaknya titik integrasi per langkah `t_inc = 20 / ts_base + 2` (pembagian bilangan bulat), lebar titik `t_inc_hr = dtm / t_inc / 60` jam.
3. Untuk setiap hari (2), setiap langkah, dan setiap titik: waktu kumulatif `t` bertambah `t_inc_hr`, lalu ordinat dihitung menurut bentuknya:

```latex
q_{seg}(t) = \begin{cases} t/t_p & t < t_p \\ (t_b - t)/(t_b - t_p) & t \ge t_p \end{cases} \qquad q_{gam}(t) = \left(\frac{t}{t_p}\right)^{\alpha} \exp\left[\left(1 - \frac{t}{t_p}\right)\alpha\right]
```

4. `q = max(q, 0)`; luas trapesium `(q + q_lalu)/2` ditambahkan ke elemen langkah dan ke jumlah total `sumq`.
5. Setelah titik-titik sebuah langkah, bila `q < 1e-4` loop langkah hari itu dihentikan (langkah sisanya hari itu tetap nol), tetapi hari berikutnya tetap dihitung.
6. Semua elemen dibagi `sumq`.

- **Kebiasaan SWAT+ / catatan port:** penghentian loop pada `q < 1e-4` hanya memutus langkah hari berjalan; hari kedua tetap diintegrasikan mulai dari `t` terakhir (didokumentasikan di `docs/subdaily.md`). Pangkat gamma memakai `rpow`. Pembagian bilangan bulat `20 / ts_base` dipertahankan.

#### 12.2.2  `Network::flow_hyd`

- **Lokasi:** `route.rs`, `fn flow_hyd(o: &mut Ob, step: usize, surfq: f32)`.
- **Padanan SWAT+:** `flow_hyd_ru_hru.f90` (dipanggil dari `hru_hyds.f90` dan `ru_control.f90`).
- **Tujuan:** menyebar limpasan permukaan hari ini (`surfq`, m3) ke langkah-langkah hari ini dan besok dengan unit hidrograf objek.
- **Masukan dan keluaran:** `o.uh`, `o.day_cur`, `surfq` (m3); mengubah `o.hyd_flo` (m3 per langkah).
- **Cara kerja:**

1. Hari "sebelumnya" (`day_cur − 1`, melingkar ke `day_max`) dikosongkan; dengan `day_max` = 2 slot ini adalah slot yang akan menjadi "besok".
2. Untuk `iday = 1..day_max` dan setiap langkah: `hyd_flo(hari_ke, langkah) += uh(iday, langkah) · surfq`, dengan `hari_ke` mulai dari `day_cur` dan maju melingkar.

```latex
Q_{d}(k) \mathrel{+}= U_1(k)\,S_q, \qquad Q_{d+1}(k) = U_2(k)\,S_q
```

- **Kebiasaan SWAT+ / catatan port:** SWAT+ memberikan juga aliran lateral dan drainase ke `flow_hyd_ru_hru` tetapi tidak menambahkannya; port hanya meneruskan limpasan permukaan. Aliran lateral dan tile mencapai saluran merata per langkah lewat `sum_tsin` (hidrograf total).

#### 12.2.3  `Network::shift`

- **Lokasi:** `route.rs`, `fn shift(hyd: &mut [f32], step: usize, cur: usize, next: usize, tinc: usize)`.
- **Padanan SWAT+:** loop pergeseran di `hru_hyds.f90` dan `ru_control.f90` (jalur Green-Ampt).
- **Tujuan:** menunda hidrograf hari `cur` sebanyak `tinc` langkah; langkah yang melewati tengah malam pindah ke awal hari `next`.
- **Cara kerja:**

1. Untuk `istep = 1..tinc`: `hyd(next, istep) = hyd(cur, step − tinc + istep)`.
2. Dari belakang (`bak = step..1`): `hyd(cur, bak) = hyd(cur, bak − tinc)` bila `bak > tinc`, selain itu 0.

- **Kebiasaan SWAT+ / catatan port:** urutan dari belakang dipertahankan supaya tidak menimpa nilai yang belum dipindahkan; `tinc` dibatasi pada `[0, step]` oleh pemanggil.

#### 12.2.4  `Network::next_day`

- **Lokasi:** `route.rs`, `fn next_day(o: &mut Ob)`.
- **Padanan SWAT+:** pengaturan `ob%day_cur` di `command.f90`.
- **Tujuan:** memajukan hari hidrograf aktif sebuah objek.
- **Cara kerja:** HRU dan RU: `day_cur += 1`; jenis lain: `day_cur = 2`. Bila melebihi `day_max`, kembali ke 1.
- **Kebiasaan SWAT+ / catatan port:** objek non-HRU/RU dengan `day_max` = 2 (saluran, outlet) selalu berada di hari 2, sedangkan saluran menulis keluarannya di slot hari 1 (`ch_rtmusk` mengosongkan seluruh hidrograf lalu mengisi hari pertama) dan pembacanya mengambil slot hari 1 juga. Port mereproduksi konvensi ini apa adanya.

#### 12.2.5  `Network::sum_tsin`

- **Lokasi:** `route.rs`, `fn sum_tsin(&mut self, icmd: usize)`.
- **Padanan SWAT+:** bagian `tsin` di `command.f90`.
- **Tujuan:** menjumlahkan inflow per langkah (`tsin`, m3 per langkah) objek `icmd` dari seluruh pengirimnya.
- **Masukan dan keluaran:** `ins` objek, hidrograf dan `hyd_flo` pengirim; keluaran `obs[icmd].tsin`.
- **Cara kerja:** untuk setiap inflow (dalam urutan `ins`), hidrograf per langkah `h(k)` dipilih menurut jenis pengirim, lalu `tsin(k) += frac · h(k)`:

| Pengirim | `ihtyp` | `h(k)` |
| --- | --- | --- |
| HRU, RU | 1 (tot) | `hyd_flo(day_cur, k) + (Q_lat + Q_til)/step` |
| HRU, RU | 3 (sur) | `hyd_flo(day_cur, k)` |
| HRU, RU | 2, 4, 5 | `hd(ihtyp).flo / step` (merata) |
| akuifer, reservoir, outlet | apa pun | `hd(ihtyp).flo / step` |
| saluran SWAT-DEG | apa pun | `hyd_flo(1, k)` |
| recall | apa pun | `hd(1).flo / step` |
| jenis lain | — | larik dari inflow sebelumnya (tidak di-reset) |

- **Kebiasaan SWAT+ / catatan port:** larik kerja `hyd` tidak dinolkan antar-inflow, sehingga jenis yang tidak dikenali meneruskan hidrograf inflow sebelumnya (dicatat di dokumentasi fungsi). Untuk recall, port memakai sebaran merata dari debit harian; jalur recall sub-harian SWAT+ (`recall%hyd_flo`) tidak ikut diporting.

#### 12.2.6  Fungsi akses kecil

| Fungsi | Lokasi | Kegunaan |
| --- | --- | --- |
| `Network::hyd_flo_day1` | `route.rs` | irisan `hyd_flo` hari pertama objek (padanan `hyd_flo(1,:)`), untuk keluaran sub-harian saluran |
| `Network::hru_order` | `route.rs` | daftar indeks HRU (0-based) dalam urutan perintah SWAT+ |

### 12.3  Routing lanskap antar-HRU (`route.rs` dan `docs/routing.md`)

HRU yang baris `hru.con`-nya mengirim air ke HRU lain (hidrograf `tot`, `sur`, `lat` atau `til`) merouting air itu melintasi lahan: HRU penerima mendapatkannya pada hari yang sama, sebelum evapotranspirasinya sendiri. Karena urutan perintah (12.1.1 langkah 8) menempatkan HRU hulu sebelum HRU hilir, cukup memanggil HRU dalam `hru_order`. Tabel keluaran HRU tetap ditulis dalam urutan nomornya. Sisi penerima (`rls_routesurf`, `rls_routesoil`) diimplementasikan di `hru/mod.rs` (bab HRU); ringkasannya di sini untuk kelengkapan:

- **Runon permukaan ke HRU kering adalah hujan:** ditambahkan ke presipitasi efektif hari itu (dan ke tiap langkah, dibagi rata, untuk run sub-harian), sehingga ia terinfiltrasi atau melimpas bersama hujan HRU itu. Sedimennya diteruskan sampai kapasitas angkut:

```latex
T_c = dep_{co}\cdot C_{USLE}\cdot q_{ls}\cdot S^{1.4}\cdot W_{field}^{1.4}, \qquad q_{ls} = q_{sur} + \frac{q_{til}}{10\,A}
```

- **Runon lateral masuk ke tanah,** ke setiap lapisan sebanding dengan tebalnya: `ΔSW_l = (tebal_l / kedalaman_profil) · q_lat`.
- **HRU wetland** memasukkan runon permukaan + tile langsung ke kolamnya (per hektar ke dalam tampungan m3; kuirk yang dicatat di `docs/routing.md`), dan runon yang sama ikut terinfiltrasi di `swr_percmain`.

Kuirk yang dipertahankan (dari `docs/routing.md`): runon tile dibagi luas dua kali (`rls_routesurf` menambahkan `hin_til%flo / (10·area)` padahal `command` sudah mengubahnya ke mm); air dari objek lain ke HRU atau ke RU ditolak oleh port (SWAT+ menambahkannya ke runon permukaan HRU, atau kehilangannya di RU). Validasi: skenario **Ps\_land** (tiga HRU kelapa sawit di hulu dialirkan ke hutan rawa gambut dan ke lahan kering) cocok dengan SWAT+ untuk semua tabel.

#### 12.3.1  `Network::hru_runon`

- **Lokasi:** `route.rs`, `pub fn hru_runon(&self, j: usize) -> Option<crate::hru::Runon>`.
- **Padanan SWAT+:** bagian HRU penerima di `command.f90` (`ob%hin_sur`, `hin_lat`, `hin_til`).
- **Tujuan:** menghitung air dan muatan yang masuk ke HRU `j` dari HRU lain, per hektar HRU penerima.
- **Masukan dan keluaran:** `ins` HRU `j` dan hidrograf `hd` pengirim (m3, t, kg); keluaran `Runon { sur, lat, til }` dengan `flo` dalam mm dan massa per ha. `None` bila tidak ada inflow.
- **Cara kerja:**

1. Untuk setiap inflow: `tot` (1) dipecah menjadi `hd(3)` ke permukaan, `hd(5)` ke tile, `hd(4)` ke lateral, masing-masing dikali `frac`; `sur` (3), `lat` (4), `til` (5) langsung ke komponennya; `rhg` (2) diabaikan.
2. Setiap jumlah dibagi luas penerima (ha), dan debitnya dibagi lagi 10 (m3/ha → mm):

```latex
q_{x} = \frac{\sum_i f_i\,Q_{x,i}}{10\,A_{j}}\ \text{(mm)}, \qquad m_{x} = \frac{\sum_i f_i\,M_{x,i}}{A_j}\ \text{(per ha)}
```

- **Kebiasaan SWAT+ / catatan port:** urutan penjumlahan (sur, tile, lat untuk `tot`) mengikuti `command`. Recharge yang dikirim ke HRU diabaikan, sesuai SWAT+.

#### 12.3.2  `Network::set_hru`

- **Lokasi:** `route.rs`, `pub fn set_hru(&mut self, j: usize, hd: &[Hyd; 5], hhsurfq: &[f32])`.
- **Padanan SWAT+:** `hru_hyds.f90` (penyalinan hidrograf dan hidrograf sub-harian), pemajuan `day_cur` di `command.f90`.
- **Tujuan:** mencatat lima hidrograf HRU `j` hari ini ke jaringan dan, untuk run sub-harian, membangun `hyd_flo`-nya.
- **Masukan dan keluaran:** `hd` (tot, rhg, sur, lat, til; m3, t, kg), `hhsurfq` (limpasan per langkah setelah lag, mm); mengubah `obs[j]` dan `hhsurfq[j]`.
- **Cara kerja:**

1. Salin `hd`, panggil `next_day`.
2. Sub-harian dengan bilangan kurva: `flow_hyd(o, step, hd(3).flo)`.
3. Sub-harian dengan Green-Ampt: tambahkan limpasan per langkah ke hari ini, kosongkan besok, lalu geser bila waktu konsentrasi lebih panjang dari satu langkah:

```latex
Q_{cur}(k) \mathrel{+}= \frac{hhsurfq(k)\cdot 10\,A}{dtm}\cdot 60, \qquad t_{inc} = \min\left(step,\ \left\lfloor\frac{60\,t_c}{dtm}\right\rfloor\right)\ \text{bila}\ 60\,t_c > dtm
```

4. Simpan salinan `hhsurfq` untuk dipakai `ru_control`.

- **Kebiasaan SWAT+ / catatan port:** faktor konversi `·10A/dtm·60` diambil apa adanya dari `hru_hyds` (satuan efektifnya tidak konsisten dengan label tabel; lihat `docs/subdaily.md`, `channel_sd_subday` berisi volume per langkah walau berlabel m3/s).

### 12.4  Loop perintah dan unit routing (`route.rs`)

#### 12.4.1  `Network::day`

- **Lokasi:** `route.rs`, `pub fn day(&mut self, nd: &NetDay)`.
- **Padanan SWAT+:** `command.f90` (bagian setelah HRU), dengan panggilan ke `ru_control`, `aqu_1d_control`, `sd_channel_control3`, `res_control`, dan pembacaan recall (`recall_*` di `command`).
- **Tujuan:** menjalankan satu hari semua objek di luar HRU dalam urutan perintah.
- **Masukan dan keluaran:** `NetDay` (cuaca per stasiun, panjang hari, pengukur suhu, neraca air HRU, tanggal); mengubah `hd`, `hin`, `hyd_flo`, `tsin` setiap objek serta keadaan RU, akuifer, saluran, reservoir, dan recall.
- **Cara kerja:** untuk setiap objek `icmd` di `order`:

1. Inflow harian: untuk setiap `In` dalam urutan, `ht1 = frac · hd_from(ihtyp)`; `hin = Σ ht1` dengan `Hyd::plus` (suhu tercampur tertimbang debit); setiap `ht1` disimpan di `hin_d` (padanan `ob%hin_d`).

```latex
H_{in} = \sum_{i \in ins} f_i \cdot H_{from_i}^{(ihtyp_i)}
```

2. `next_day`; bila sub-harian, `sum_tsin`.
3. Pilih menurut jenis:
   - **RU:** `ru_control` (12.4.2).
   - **Akuifer:** `aqu_1d_control` dengan PET stasiun (Hargreaves stasiun; Bab 14).
   - **Saluran SWAT-DEG** bila `channels` ada: susun `ChanCtx` (cuaca, panjang hari, pengukur suhu, `yrs`, `jday`, `hru_wb`) dan `ChanInflows` (asal inflow, `hin_d`, RU inflow), panggil `Channels::control`, simpan hasilnya di `hd(1)`. Hidrograf `hyd_flo` objek dikosongkan seluruhnya lalu slot hari pertama diisi dengan keluaran per langkah (padanan perilaku `ch_rtmusk`). Bila stasiun tidak ada, objek dilewati.
   - **Reservoir:** hanya bila memiliki inflow; `Reservoir::control(hin, yrc, mo, jday, day_mo, precip, pet, tave, print)` (bab reservoir).
   - **Recall:** `Recall::day(jday, mo, yrc, yrs)` memberi beban hari ini sebagai `hd(1)` (bab reservoir/recall).
   - **Lainnya** (outlet; saluran SWAT-DEG bila tidak dirouting): `hd(1) = hin`.

- **Kebiasaan SWAT+ / catatan port:** reservoir tanpa inflow tidak dijalankan sama sekali (sesuai `command`). Ketika saluran tidak diporting untuk proyek tertentu (lihat `channel_skipped`), saluran SWAT-DEG menjadi penerus murni, perilaku yang sama dengan saluran SWAT+ yang panjangnya kurang dari 1 m.

#### 12.4.2  `Network::ru_control`

- **Lokasi:** `route.rs`, `fn ru_control(&mut self, iru: usize)`.
- **Padanan SWAT+:** `ru_control.f90`.
- **Tujuan:** menjumlahkan hidrograf elemen-elemen RU (HRU), dikalikan *delivery ratio* dan fraksi, menjadi lima hidrograf RU; menghitung `ru_d` untuk keluaran `ru_day`; dan hidrograf sub-harian RU.
- **Masukan dan keluaran:** elemen (`frac`, `dr`), `hd` setiap HRU elemen, `hhsurfq`; keluaran `obs[ru].hd` (5 hidrograf), `ru_d[iru]`, `obs[ru].hyd_flo`.
- **Cara kerja:**

1. Untuk setiap elemen: fraksi efektif `ef = frac`; untuk HRU dengan `frac < 0,99999` fraksi diubah dari bagian luas RU menjadi bagian luas HRU:

```latex
ef = frac \cdot \frac{A_{RU}\,[\mathrm{km^2}]}{A_{HRU}\,[\mathrm{ha}]/100}
```

2. Untuk setiap hidrograf `k` (tot, rhg, sur, lat, til): `ht1 = hd_k ** DR` (perkalian komponen) kecuali recharge (`k` = 2, tanpa DR); `ht1 = ef · ht1`; `hd_RU(k) += ht1`.

```latex
H_{RU}^{(k)} = \sum_e ef_e \cdot \left(H_e^{(k)} \odot DR_e\right), \quad k \ne 2; \qquad H_{RU}^{(2)} = \sum_e ef_e\,H_e^{(2)}
```

3. `ru_d` = jumlah kelima `ht1` (mode asli) atau hanya tot dan rhg (`--fix-swat`).
4. Sub-harian Green-Ampt, setiap elemen HRU: hidrograf hari ini RU **diganti** dengan hidrograf HRU elemen ditambah limpasan per langkahnya,

```latex
Q_{RU,cur}(k) = Q_{HRU,cur}(k) + \frac{frac \cdot A_{RU}\,[ha]\cdot DR_{flo}\cdot 10}{dtm \cdot 60}\, hhsurfq(k)
```

Setelah itu hari besok dikosongkan dan hidrograf digeser dengan waktu konsentrasi tetap `ru_tc = 2` jam (`tinc = int(120/dtm)` bila `120 > dtm`).

5. Salin lima hidrograf ke objek RU. Sub-harian dengan bilangan kurva: `flow_hyd(RU, step, hd_RU(3).flo)`.

- **Kebiasaan SWAT+ / catatan port:** (a) `ru_d` menjumlahkan kelima hidrograf padahal `tot` sudah berisi sur+lat+til, sehingga aliran itu terhitung dua kali; dikoreksi dengan `--fix-swat` (hanya tot dan rhg; `docs/swat-fixes.md`, efek di Bh\_land: debit RU berkurang separuh). (b) Pada Green-Ampt, hidrograf RU adalah milik HRU terakhir (tiap elemen menimpa), dan waktu konsentrasinya selalu 2 jam (`docs/subdaily.md`). (c) Recharge tidak dikenai DR.

#### 12.4.3  Fungsi akuifer (rujukan ke Bab 14)

| Fungsi | Lokasi | Kegunaan |
| --- | --- | --- |
| `AquDay::columns` | `route.rs` | 17 kolom `aquifer_day.txt` dengan label asli SWAT+ — Bab 14 |
| `AquDay::columns_as` | `route.rs` | idem, label `cbn`/`orgn` terkoreksi bila `fixes` — Bab 14 |
| `Network::aqu_1d_control` | `route.rs` | `aqu_1d_control.f90`: recharge, aliran balik resesi, rembesan, revap, nitrat — Bab 14 |
| `Network::aquifers` | `route.rs` | iterator (nomor, GIS id, nama, `AquDay`) untuk keluaran — Bab 14 |

Bagian `aqu_initial` dan kalibrasi akuifer (`cal_conditions`) di dalam `Network::new` juga diuraikan di Bab 14.

#### 12.4.4  Fungsi akses keluaran jaringan

| Fungsi | Lokasi | Kegunaan |
| --- | --- | --- |
| `Network::channels` | `route.rs` | daftar `ChannelDay` per saluran SWAT-DEG (nomor 1-based, GIS id, nama, `ChWat`, tampungan, inflow, outflow dalam m3/s, suhu air dari `Channels::wtemp_of`); kosong bila saluran tidak dirouting |
| `Network::reservoir_days` | `route.rs` | iterator (nomor, GIS id, nama, `&Reservoir`) |
| `Network::recall_days` | `route.rs` | iterator (nomor, nama, beban hari ini `&Hyd`) |
| `Network::channel_steps` | `route.rs` | untuk run sub-harian: keluaran per langkah tiap saluran (`hyd_flo(1,:)`, m3 per langkah) untuk `channel_sd_subday.txt` |
| `Network::routing_units` | `route.rs` | iterator (nama, `ru_d`) untuk `ru_day` |

### 12.5  Saluran SWAT-DEG: struktur data dan fungsi bantu (`channel.rs`)

`channel.rs` memporting saluran SWAT-DEG (`chandeg.con`, `channel-lte.cha`, `hyd-sed-lte.cha`, `nutrients.cha`) dengan algoritma `sd_channel_control3`: kurva rating empat kedalaman, routing tampungan (Muskingum bila `codes.bsn` `rte` = 1, *variable storage coefficient* bila `rte` = 0), deposisi dataran banjir dan erosi tebing, rembesan dan penguapan, kualitas air QUAL2E, serta suhu air sungai. Pestisida, patogen, garam/konstituen, gwflow, dan alokasi air (`wallo`) tidak diporting. Struct utamanya:

| Struct | Isi utama (satuan) |
| --- | --- |
| `Rc` (publik) | satu titik kurva rating (`channel_rating_curve_parameters`): `flo_rate` (m3/s), `xsec_area` (m2), `surf_area` (m·km), `dep` (m), `top_wid` (m), `vol`, `vol_fp`, `vol_ch` (m3), `wet_perim` (m), `vel` (m/s), `ttime` (jam) |
| `ChNut` | laju QUAL2E saluran (`channel_nut_data` setelah default `ch_read_nut`): `rs1`..`rs5` (pengendapan alga, sumber P dan NH4 bentik, pengendapan N dan P organik), `rk1`..`rk4` (peluruhan dan pengendapan CBOD, reaerasi, kebutuhan oksigen sedimen), `bc2`..`bc4` (nitrifikasi NO2, hidrolisis N organik, mineralisasi P organik), `igropt`, rasio stoikiometri alga `ai0`, `ai2`..`ai6`, `mumax`, `rhoq`, `tfact`, konstanta setengah jenuh `k_l`, `k_n`, `k_p`, koefisien peneduhan `lambda0..2`, preferensi amonium `p_n` |
| `ChWat` (publik) | badan air harian (`ch_wat_d`): luas `area_ha` (ha), hujan `precip`, penguapan `evap`, rembesan `seep` (m3) |
| `SdCh` | satu saluran: geometri `chw` (lebar, m), `chd` (kedalaman, m), `chs` (kemiringan, m/m), `chl` (panjang, km), `chn` (n Manning), `chk` (konduktivitas dasar, mm/jam), `cov` (faktor penutupan), `sinu` (sinuositas), `vcr_coef`, `ch_clay` (%), `ch_bd` (t/m3), `n_conc`, `p_conc` (mg/kg), `p_bio`; parameter `sed_nut` (`pk_rto`, `n_sol_part`, `p_sol_part`, `n_dep_enr`, `p_dep_enr`, `wash_bed_fr`); `db_chw`, `db_chd` (lebar/kedalaman dari baris data bernomor sama dengan saluran); `nut`; kurva `elev[4]`; titik rating `in1`, `in2`, `out1`, `out2`; koefisien Muskingum `msk_c[3]`, `substeps`, `nsteps`; volume `in1_vol`, `out1_vol` (m3); tampungan `tot_stor`, `ch_stor`, `fp_stor` (`Hyd`); `vel` (m/s), `rttime` (jam); `wat`, `ch_in`, `ch_out`, `wtemp` |
| `ChanCtx` (publik) | konteks hari: cuaca stasiun, panjang hari `daylth` (jam), pengukur suhu `tgage`, `yrs`, `jday`, `hru_wb` |
| `ChanInflows` (publik) | asal inflow untuk `ch_temp`: `from_chandeg` (per inflow), `hin_d`, `rus` |
| `Basin` | `rte`, `qual2e`, `evrch` (koef. penguapan saluran, default 0,6), `scoef` (default 1), `fixes` |
| `Channels` (publik) | semua saluran `chs`, `b`, `gra` (laju tumbuh alga yang tersimpan antarpanggilan, atribut *save* Fortran), `wtemp` (satu nilai untuk semua saluran), `ru_hrus` dan `ru_elem_is_hru` (pandangan `ch_temp` terhadap RU), `step`, `dtm`, `inflo` (inflow langkah tersimpan, atribut *save*) |

#### 12.5.1  `Rc::scale`

- **Lokasi:** `channel.rs`, `fn scale(self, k: f32) -> Rc`.
- **Padanan SWAT+:** operator `chrc_mult` (`rc * const`) di `sd_channel_module.f90`.
- **Tujuan:** mengalikan semua 11 field titik rating dengan konstanta.
- **Cara kerja:** setiap field dikali `k`, dalam urutan field Fortran (xsec\_area, surf\_area, flo\_rate, dep, top\_wid, vol, vol\_ch, vol\_fp, wet\_perim, vel, ttime).

#### 12.5.2  `Rc::interp`

- **Lokasi:** `channel.rs`, `fn interp(a: Rc, b: Rc, k: f32) -> Rc`.
- **Padanan SWAT+:** operator `chrc_interp` di `sd_channel_module.f90`.
- **Tujuan:** interpolasi linear antara dua titik kurva rating.
- **Cara kerja:** untuk setiap field:

```latex
x = x_a + k\,(x_b - x_a)
```

- **Kebiasaan SWAT+ / catatan port:** bentuk `a + k(b − a)` (bukan `(1−k)a + kb`) dipertahankan karena memengaruhi pembulatan f32.

#### 12.5.3  `qman`

- **Lokasi:** `channel.rs`, `fn qman(x1: f32, x2: f32, x3: f32, x4: f32) -> f32`.
- **Padanan SWAT+:** `qman.f90`.
- **Tujuan:** persamaan Manning: debit (bila `x1` luas penampang) atau kecepatan (bila `x1 = 1`).
- **Masukan dan keluaran:** `x1` luas (m2) atau 1, `x2` jari-jari hidraulik R (m), `x3` n Manning, `x4` kemiringan (m/m); keluaran m3/s atau m/s.

```latex
Q = \frac{A\,R^{0.6666}\,\sqrt{S}}{n + 0.001}
```

- **Kebiasaan SWAT+ / catatan port:** eksponen 0,6666 (bukan 2/3) dan tambahan 0,001 pada n dipertahankan; pangkat via `rpow`.

#### 12.5.4  `theta`

- **Lokasi:** `channel.rs`, `fn theta(r20: f32, thk: f32, tmp: f32) -> f32`.
- **Padanan SWAT+:** `theta.f90`.
- **Tujuan:** koreksi suhu laju reaksi orde satu (bentuk Arrhenius yang lazim di QUAL2E).

```latex
k_T = k_{20}\,\theta^{\,T-20}
```

#### 12.5.5  `wq_semianalyt`

- **Lokasi:** `channel.rs`, `fn wq_semianalyt(tres, tdel, term_m, prock, cprev, cint) -> f32`.
- **Padanan SWAT+:** `wq_semianalyt.f90`.
- **Tujuan:** solusi analitis konsentrasi di reaktor tercampur sempurna dengan waktu tinggal `tres`, laju orde satu `prock`, suku sumber tetap `term_m`, konsentrasi awal `cprev` dan konsentrasi inflow `cint`, setelah selang `tdel`.
- **Cara kerja:** persamaan diferensial dan solusinya:

```latex
\frac{dc}{dt} = \frac{c_{in} - c}{\tau} + k\,c + m \;\Rightarrow\; c(\Delta t) = c_0\,e^{-\alpha \Delta t} + \frac{c_{in}/\tau + m}{\alpha}\left(1 - e^{-\alpha \Delta t}\right), \quad \alpha = \frac{1}{\tau} - k
```

- **Kebiasaan SWAT+ / catatan port:** eksponen memakai `exp_w` (argumen dibatasi ±80 seperti `Exp_w` Fortran). Urutan suku `help1..help4` sama dengan Fortran.

#### 12.5.6  `wq_k2m`

- **Lokasi:** `channel.rs`, `fn wq_k2m(t1, t2, tk, c1, c2) -> f32`.
- **Padanan SWAT+:** `wq_k2m.f90`.
- **Tujuan:** mengubah proses orde satu (laju `tk`) menjadi suku sumber/serap konstan setara `m` (mg/L per hari), supaya proses itu dapat dimasukkan ke persamaan konsentrasi spesies lain (mis. CBOD yang meluruh menjadi kebutuhan oksigen).
- **Cara kerja:** hitung `c_k = wq_semianalyt(t1, t2, 0, tk, c1, c2)` lalu cari `m` yang memberi konsentrasi akhir sama tanpa reaksi:

```latex
m = \frac{c_k - c_1\,e^{-t_2/t_1}}{t_1\left(1 - e^{-t_2/t_1}\right)} - \frac{c_2}{t_1}
```

#### 12.5.7  `ChNut::new`

- **Lokasi:** `channel.rs`, `fn new(t: Option<&Table>, row: Option<usize>, step: f32) -> ChNut`.
- **Padanan SWAT+:** `ch_read_nut.f90` (nilai default bila ≤ 0).
- **Tujuan:** membaca laju QUAL2E dari `nutrients.cha` untuk satu saluran, mengganti nilai tak-positif dengan default, dan membagi laju dengan banyaknya langkah per hari.
- **Cara kerja:**

1. Setiap field dibaca dari baris tabel; bila tidak ada atau ≤ 0 diganti default: `igropt` 2, `ai0` 50, `ai2` 0,015, `ai3` 1,60, `ai4` 2,0, `ai5` 3,5, `ai6` 1,07, `mumax` 2,0, `rhoq` 2,5, `tfact` 0,3, `k_l` 0,75, `k_n` 0,02, `k_p` 0,025, `lambda0` 1,0, `lambda1` 0,03, `lambda2` 0,054, `p_n` 0,5, `rs1` 1,0, `rs2` 0,05, `rs3` 0,5, `rs4` 0,05, `rs5` 0,05, `rk1` 1,71, `rk2` 1,0, `rk4` 2,0, `bc2` 1,1, `bc3` 0,21, `bc4` 0,35. `rk3` tidak diganti bila ≤ 0 (default hanya bila tidak ada: 2,0).
2. `k_l` diubah satuannya: `k_l · 1e-3 · 60` (dari kJ/(m2·menit) ke MJ/(m2·jam)).
3. `mumax`, `rhoq`, `rs1..rs5`, `rk1..rk4`, `bc2..bc4` dibagi `step`.

- **Kebiasaan SWAT+ / catatan port:** laju dibagi langkah per hari, tetapi QUAL2E tetap dijalankan sekali sehari, sehingga pada run sub-harian alga, nitrogen, dan oksigen berubah `step` kali lebih lambat (`docs/subdaily.md`). Baris yang dipakai adalah baris data **bernomor sama dengan saluran** (`sd_dat(ich)%nut`), bukan pointer `channel-lte.cha` saluran itu (`docs/swat-quirks.md`); keduanya sama bila, seperti tulisan SWAT+ Editor, ada satu baris per saluran.

### 12.6  Inisialisasi saluran dan kurva rating (`channel.rs`)

#### 12.6.1  `Channels::new`

- **Lokasi:** `channel.rs`, `pub fn new(project: &Project, props: &[i32], fixes: bool) -> Result<Channels, String>`.
- **Padanan SWAT+:** `sd_channel_read.f90`, `sd_hydsed_init.f90` (termasuk koefisien Muskingum), `sd_rating_curve.f90`, `ch_read_nut.f90`; kondisi awal dari `ch_init`/`om_water` (`initial.cha`).
- **Tujuan:** menyiapkan setiap saluran SWAT-DEG dalam urutan `chandeg.con`: parameter hidraulik-sedimen, kurva rating, koefisien Muskingum, laju QUAL2E, dan keadaan awal; serta pandangan `ch_temp` terhadap RU.
- **Masukan dan keluaran:** `project` (tabel `hyd-sed-lte.cha`, `channel-lte.cha`, `nutrients.cha`, `initial.cha`, `om_water.ini`, `codes.bsn`, `parameters.bsn`, `time.sim`, definisi RU), `props` (pointer data per saluran), `fixes`. Keluaran `Channels` atau `Err` dengan nama fitur yang belum diporting: laju lapse (`lapse` = 1), file `temperature.cha`, tidak ada `hyd-sed-lte.cha`, air awal saluran (`flo > 0` di `om_water`), atau saluran dengan panjang ≤ 1e-3 km.
- **Cara kerja:**

1. Parameter cekungan: `rte`, `qual2e`, `evrch` (default 0,6), `scoef` (default 1).
2. Untuk saluran ke-`ich` (1-based): baris data `dat = sd_channel_data[props]`, baris hidraulik `hyd-sed-lte.cha` dicari dengan nama `dat.hyd`. Baris QUAL2E dicari dari nama `nut` milik baris data **bernomor `ich`**, dan `db_chw`, `db_chd` dari baris `hyd-sed-lte.cha` ke-`ich` (kuirk `sd_dat(ich)`, `sd_chd(ich)`).
3. Field `chw, chd, chs, chl, chn, chk, cov, sinu, vcr_coef, ch_clay, ch_bd, n_conc, p_conc, p_bio` dibaca; `sed_nut.cha` tidak dibaca sehingga default tipe berlaku: `pk_rto` 1, `n_sol_part` 0,01, `p_sol_part` 0,01, `n_dep_enr` 0,5, `p_dep_enr` 0,5, `wash_bed_fr` 0,1.
4. Batas bawah: `chs < 1e-9 → 1e-6`, `chn < 0,01 → 0,01`, `sinu < 1,05 → 1,05`. Kemiringan dataran banjir `fps` dibatasi `≤ chs` lalu `≥ 1e-5` (bila ≤ 1e-6); `fpn` adalah n Manning dataran banjir; `chss` kemiringan sisi.
5. **Kurva rating dalam alur** (`sd_rating_curve`). Lebar dasar `b = chw − 2·chd·z`; bila ≤ 0 maka `b = max(0, 0,5·chw)` dan `z = (chw − b)/(2·chd)`. Untuk kedalaman `d = 0,1·chd` (titik 1) dan `d = chd` (titik 2, *bankfull*):

```latex
\text{asli: } P = b + 2\sqrt{d^2\left(1 + \frac{1}{z^2}\right)},\quad A = b\,d + \frac{d}{z} \qquad \text{koreksi: } P = b + 2d\sqrt{1+z^2},\quad A = b\,d + z\,d^2
```

```latex
R = \frac{A}{P},\quad W_{top} = b + 2dz,\quad V = 1000\,A\,L,\quad Q = \frac{A R^{0.6666}\sqrt{S}}{n+0.001},\quad v = \frac{R^{0.6666}\sqrt{S}}{n+0.001},\quad TT = \frac{L}{3.6\,v}
```

dengan L = `chl` (km), sehingga V dalam m3 dan TT dalam jam; `surf_area = W_top·L`, `vol_ch = V`, `vol_fp = 0`. Nilai P, A, dan `vol_ch` bankfull disimpan.

6. **Kurva rating dataran banjir.** Untuk kedalaman di atas bankfull `d = 0,2·chd` (titik 3) dan `d = chd` (titik 4):

```latex
P = P_{bf} + 2\sqrt{d^2\left(1+\frac{1}{fps^2}\right)},\quad A = A_{bf} + B\,d + \frac{d^2}{fps},\quad B = \begin{cases} b & \text{asli} \\ chw & \text{koreksi}\end{cases}
```

```latex
dep = (1+f)\,chd,\quad W_{top} = chw + \frac{2d}{fps},\quad V_{fp} = 1000\,W_{top}\,d\,L,\quad V = V_{bf} + V_{fp}
```

dengan Q dan v dari `qman` memakai `fpn`.

7. **Koefisien Muskingum** (`sd_hydsed_init`, stabilitas menurut Jeong 2011). Untuk 0,1·chd dan chd (geometri yang sama dengan langkah 5) dihitung kecepatan v, celeritas `c = 5v/3` dan waktu tampung `K_i = L/(3.6·c)` jam. Lalu:

```latex
K = \frac{co_1}{co_1+co_2}K_{bf} + \frac{co_2}{co_1+co_2}K_{0.1bf}, \qquad \Delta t_{max} = 2K(1-X)
```

dengan `msk_co1` (default 0,75), `msk_co2` (0,25), `msk_x` (0,2) dari `parameters.bsn`. Selang `Δt = dtm/60` jam; bila `Δt > Δt_max`, `substeps = int(Δt/Δt_max) + 1`; bila `rte = 0` dan run harian, `substeps = 1`. `nsteps = step·substeps`, `Δt ← Δt/substeps`, dan

```latex
D = 2K(1-X) + \Delta t,\quad C_1 = \max\left(0, \frac{\Delta t - 2KX}{D}\right),\quad C_2 = \frac{\Delta t + 2KX}{D},\quad C_3 = \frac{2K(1-X) - \Delta t}{D}
```

ketiganya dinormalisasi dengan jumlahnya sehingga `C_1 + C_2 + C_3 = 1`.

8. **Keadaan awal tanpa air:** titik rating pada kedalaman 0 (`rcurv_interp_dep`) adalah titik 1 dikali 0 tetapi dengan `ttime` titik 1; dipakai sebagai `in1` dan `out1`. `in1_vol = out1_vol = 0`.
9. Setelah semua saluran: `ru_hrus` (untuk setiap RU, daftar (HRU 0-based, fraksi) elemen) dan `ru_elem_is_hru` (tipe setiap baris tabel elemen global), `step`, `dtm`, `gra = 0`, `wtemp = 0`, `inflo = 0`.

- **Kebiasaan SWAT+ / catatan port:** (a) Luas penampang alur `b·d + d/z` dan keliling basah yang memperlakukan z sebagai vertikal/horizontal (asli) versus trapesium benar (`--fix-swat`); di Bh\_land koreksi menaikkan tampungan dan rembesan beberapa persen, debit outlet berubah < 0,1 %. (b) Di atas bankfull lebar alir memakai lebar dasar `b` (asli) versus lebar atas `chw` (koreksi). (c) Pada run harian, koefisien Muskingum dihitung untuk sub-langkah stabil, tetapi `ch_rtmusk` merouting hari itu dalam satu langkah dengan koefisien tersebut. (d) Sebelum ada aliran, titik keluaran terakhir menyimpan waktu tempuh titik terdangkal, yang menentukan koefisien tampungan hari pertama. (e) Saluran penerus SWAT+ (panjang < 1 m) belum diporting: seluruh jaringan saluran lalu menjadi penerus (lihat 12.4.1).

#### 12.6.2  `Channels::interp_flo`

- **Lokasi:** `channel.rs`, `fn interp_flo(&self, ich: usize, flo_rate: f32) -> Rc`.
- **Padanan SWAT+:** `rcurv_interp_flo.f90`.
- **Tujuan:** mencari titik kurva rating (kedalaman, luas, kecepatan, waktu tempuh, dsb.) untuk debit `flo_rate` (m3/s).
- **Cara kerja:** cari titik pertama `k` dengan `Q < Q_k`:

1. `k = 1`: titik 1 diskalakan `Q/Q_1`, tetapi `ttime` tetap milik titik 1.
2. `k > 1`: `Rc::interp(elev[k−1], elev[k], (Q − Q_{k−1})/(Q_k − Q_{k−1}))`.
3. Bila Q ≥ Q\_4: `r = 1 + (Q − Q_4)/Q_4`, titik 4 diskalakan `r` lalu `ttime` dibagi `r` (waktu tempuh dipertahankan pada nilai maksimum bankfull).

- **Kebiasaan SWAT+ / catatan port:** skala-lalu-bagi pada `ttime` dipertahankan (bukan sekadar menyalin) karena pembulatan f32.

#### 12.6.3  Fungsi akses kecil

| Fungsi | Lokasi | Kegunaan |
| --- | --- | --- |
| `Channels::output` | `channel.rs` | tampungan `ch_stor`, inflow `ch_in`, outflow `ch_out` (debit dalam m3/s) dan `ChWat` saluran untuk `channel_sd_day` |
| `Channels::wtemp_of` | `channel.rs` | suhu air yang dicetak: mode asli satu nilai `wtemp` untuk semua saluran (estimasi berbasis udara dari saluran terakhir); `--fix-swat` suhu sungai saluran itu sendiri |

### 12.7  Satu hari saluran: sedimen, routing tampungan, kehilangan air (`channel.rs`)

#### 12.7.1  `Channels::control`

- **Lokasi:** `channel.rs`, `pub fn control(&mut self, ich: usize, hin: Hyd, area_ha: f32, ins: &ChanInflows, cx: &ChanCtx, tsin_in: &[f32], hyd_flo: &mut Vec<f32>) -> Hyd`.
- **Padanan SWAT+:** `sd_channel_control3.f90`, yang memanggil `sd_channel_sediment3.f90`, `ch_rtmusk.f90`, `rcurv_interp_flo.f90`, `ch_watqual4.f90` dan `ch_temp.f90`.
- **Tujuan:** merouting satu hari inflow saluran `ich` (0-based): menambah hujan di permukaan air, deposisi dataran banjir dan erosi tebing, routing tampungan per langkah, rembesan dan penguapan, transformasi kualitas air, dan suhu air; mengembalikan hidrograf keluaran (`ob%hd(1)`).
- **Masukan dan keluaran:** `hin` (inflow harian: m3, t, kg), `area_ha` (luas tangkapan objek saluran, ha), `ins` dan `cx` (untuk `ch_temp`), `tsin_in` (inflow per langkah, m3 per langkah; diabaikan pada run harian). Keluaran: hidrograf outflow (m3/hari dsb.), `hyd_flo` berisi outflow per langkah (m3), serta keadaan saluran (`ch_in`, `ch_out` dengan debit m3/s, tampungan, `wat`).
- **Cara kerja (bagian A, persiapan):**

1. `ht1 = hin` dengan suhu `5 + 0,75·T_ave`; `wtemp` diset ke nilai yang sama. Inflow per langkah `tsin = [ht1.flo]` untuk run harian, atau salinan `tsin_in`. `ch_in = ht1` dengan debit dibagi 86 400 (m3/s).

- **Cara kerja (bagian B, `sd_channel_sediment3`, hanya bila `Q_in > 1e-6` m3):**

2. Konstanta harian yang menimpa input: debit bankfull 1,5 kali kurva, lama genangan dataran banjir 1 hari, `d50` dan eksponen dasar 1 (dua terakhir tidak dipakai). Debit puncak dan titik ratingnya:

```latex
r_{pk} = pk_{rto}\left(1 + 2.66\,A_{km^2}^{-0.3}\right),\qquad Q_{pk} = r_{pk}\,\frac{Q_{in}}{86400},\qquad TT = \frac{L}{3.6\,v(Q_{pk})}
```

Kecepatan `v` dan waktu tempuh `TT` (jam) disimpan sebagai `vel`, `rttime` (dipakai `ch_temp`).

3. Hujan di permukaan saluran: `area_ha = chl·chw/10`, `P_ch = 10·P·area_ha` (m3). Setiap `tsin` dikali `1 + P_ch/Q_in`, lalu `Q_in += P_ch`.
4. **Deposisi dataran banjir** bila `Q_pk > 1,5·Q_bf` (Q\_bf debit titik 2):

```latex
A_{fp} = 5\,chw\,L\cdot 1000,\quad \beta = 10^{-4}\frac{A_{fp}}{Q_{rc}},\quad \eta = \min\left(1,\ d_{inun}\frac{Q_{rc}}{Q_{in}/86400}\left(1 - e^{-\beta}\right)\right)
```

dengan `Q_rc` debit titik rating pada `Q_pk`. Endapan: sedimen `η·sed`, N organik `η·n_dep_enr·orgn`, P sedimen `η·p_dep_enr·sedp`; dikurangkan dari `ht1`.

5. **Erosi tebing.** Kohesi tanah tebing dari persen liat C, faktor vegetasi, faktor kerapatan, dan kecepatan kritis:

```latex
c_{coh} = \max\left(1000,\ -87.1 + 42.82C - 0.261C^2 + 0.029C^3\right),\quad f_{veg} = 4500\,cov\,e^{-5\,chd}
```

```latex
f_{bd} = \max(0.001,\ 39.24\,\rho_b - 1000),\quad v_{cr} = vcr_{coef}\,\log_{10}(2200\,chd)\sqrt{0.0004\,(f_{bd} + 0.021\,c_{coh} + f_{veg})}
```

Kecepatan di tikungan, dari jari-jari lengkung menurut sinuositas `s`:

```latex
r_c = \frac{12\,chw\,s^{1.5}}{13\,(s-1)^{0.5}},\quad v_{bend} = v\left(\frac{1}{r_c} + 1\right),\quad v_{rch} = 0.33\,v_{bend} + 0.66\,v
```

Bila `v_rch > v_cr`, laju mundur tebing (m) adalah gabungan harmonik laju berbasis kecepatan dan batas kelelahan:

```latex
E_v = 10^{-4}A_{km^2}^{-0.0858}\,chw\,\frac{1}{1 + e^{-4(v_{rch}/v_{cr} - 1)}},\quad E_{max} = 0.0002\,chw,\quad E_m = \left(\frac{1}{E_v} + \frac{1}{E_{max}}\right)^{-1}
```

selain itu `E_m = 0`. Massa tererosi (t) dengan panjang busur `0,25·L` km:

```latex
E_t = 1000\,E_m\,chd\,(0.25\,L)\,\rho_b
```

Beban erosi: `sed = E_t`, `orgn = E_t·n_conc/1000`, `sedp = (1−p_bio)·E_t·p_conc/1000`, `solp = p_bio·E_t·p_conc/1000`; ditambahkan ke `ht1`. Lalu bagian yang mengendap di dasar: `sed = wash_bed_fr·E_t`, `orgn = n_dep_enr·wash_bed_fr·orgn_ero`, `sedp = p_dep_enr·wash_bed_fr·sedp_ero`, dikurangkan dari `ht1`.

- **Cara kerja (bagian C, `ch_rtmusk`):**

6. `hyd_flo` disetel ulang ke `step` nol; `wat.evap = wat.seep = 0`. Pada run harian `substeps = nsteps = 1`, selain itu nilai dari inisialisasi. `dts = dtm/substeps·60` detik, `Δt = dts/3600` jam.
7. Untuk setiap sub-langkah (penghitung `irtstep` naik setiap `substeps` sub-langkah): bila `Q_in > 1e-6`, `I = tsin(irtstep)/substeps` (disimpan di `Channels::inflo`) dan tampungan menerima bagian inflow yang sebanding, `tot_stor += min(1, max(0, I/Q_in))·ht1`. Titik rating inflow `in2 = interp_flo(I/86400)`.
8. Bila `tot_stor.flo < 1e-6`: `in1`, `out1` dan `in1_vol`, `out1_vol` di-nol-kan. Selain itu, outflow O (m3) dihitung:

```latex
\text{Muskingum } (rte=1):\ O = \min\left(V,\ \max\left(0,\ C_1 I + C_2 I_{prev} + C_3 O_{prev}\right)\right)
```

```latex
\text{VSC } (rte=0):\ SC = \min\left(1,\ \frac{scoef \cdot 2\Delta t}{2\,TT_{out1} + \Delta t}\right),\quad O = SC\cdot V
```

dengan V = `tot_stor.flo` (m3), `I_prev = in1_vol`, `O_prev = out1_vol` (diperbarui setelah hitungan Muskingum), dan `TT_out1` waktu tempuh titik rating outflow sebelumnya (jam). O tak-hingga/NaN diganti 0. Titik rating outflow `out2 = interp_flo(O/dts)`.

9. Bagian tampungan yang keluar `ρ = min(1, O/V)`: `ht2 += ρ·tot_stor` (semua konstituen keluar sebanding air), `hyd_flo(irtstep) += O`, `tot_stor ← (1−ρ)·tot_stor`, `in1 = in2`, `out1 = out2`.
10. Pembagian tampungan: bila `V > vol_ch` bankfull, fraksi `(V − vol_ch)/V` menjadi `fp_stor` dan sisanya `ch_stor`; selain itu semuanya `ch_stor`. `tot_stor = ch_stor + fp_stor`.
11. Setelah semua sub-langkah, **kehilangan transmisi** (rembesan) dan **penguapan** dari tampungan alur:

```latex
TL = \min\left(V_{ch},\ chk\cdot L\cdot chw\cdot 24\right),\qquad E = \min\left(V_{ch},\ evrch\cdot PET\cdot\frac{1000\,L\,chw}{1000}\right)
```

(chk mm/jam, L km, chw m, PET mm; hasil m3). Rembesan mengurangi semua konstituen `ch_stor` secara proporsional; penguapan hanya airnya. `wat.seep = TL`, `wat.evap = E`, `tot_stor = ch_stor + fp_stor`.

- **Cara kerja (bagian D, kualitas air, hanya bila `Q_in > 1e-6`):**

12. Titik rating untuk waktu tempuh QUAL2E: `interp_flo(0)` (asli, karena `ht3%flo` tidak pernah diisi) atau `interp_flo(Q_in/86400)` (`--fix-swat`); panggil `ch_watqual4`.
13. Bila `qual2e` = 1: pertukaran organik–mineral selama waktu tempuh pada debit inflow, dihitung ulang dari inflow:

```latex
\Delta N = \min\left(NO_3^{in},\ \left(1 - e^{-k_{N}\,TT}\right)orgN^{in}\right),\quad orgN^{out} = orgN^{in} + \Delta N,\quad NO_3^{out} = NO_3^{in} - \Delta N
```

dan serupa untuk P (`p_sol_part`, sedp/solp).

- **Cara kerja (bagian E, suhu dan keluaran):**

14. `hd1 = ht2`; `tw = ch_temp(...)`. Pada mode asli suhu outflow tetap suhu hasil campuran/`ch_watqual4` (berbasis udara); dengan `--fix-swat` `hd1.temp = tw`. `ch_out = hd1` dengan debit m3/s. Kembalikan `hd1`.

- **Kebiasaan SWAT+ / catatan port:** (a) `sd_channel_sediment3` menimpa setiap hari debit bankfull, lama genangan, `d50` dan eksponen dasar dengan konstanta. (b) Rasio `bank_ero.flo/ht1.flo` yang dipakai mengoreksi `tsin` selalu 0 (erosi tidak membawa air). (c) Endapan dataran banjir dan dasar hanya dikurangkan dari inflow; port tidak menyimpannya ke kolam lain. (d) `inflo` adalah variabel *save*: saluran tanpa inflow memakai inflow langkah terakhir yang dihitung, dari saluran mana pun, untuk titik rating inflow dan Muskingum (`docs/subdaily.md`). (e) Titik rating inflow memakai volume langkah dibagi 86 400, sedangkan outflow dibagi `dts`. (f) Hanya air dan sedimen yang benar-benar dirouting; nutrien, alga, CBOD, dan oksigen dari `ch_watqual4` dihitung dari konsentrasi inflow dan **volume inflow**, bukan dari tampungan ("water quality ignores the routing", `docs/swat-quirks.md`). (g) Pada mode asli suhu sungai hanya sampai ke `ch_stor.temp` karena outflow sudah disalin sebelum `ch_temp` dijalankan.

### 12.8  Kualitas air QUAL2E di saluran (`channel.rs`)

#### 12.8.1  `Channels::ch_watqual4`

- **Lokasi:** `crates/swat-engine/src/channel.rs`, `fn ch_watqual4(&mut self, ich: usize, ht1: &Hyd, ht2: &mut Hyd, rc_prev: Rc, cx: &ChanCtx)`.
- **Padanan SWAT+:** `ch_watqual4.f90` (QUAL2E: Brown dan Barnwell 1987; formulasi SWAT di Neitsch et al. 2011 bab 7:3), memakai `theta.f90`, `wq_semianalyt.f90`, `wq_k2m.f90`.
- **Tujuan:** mengubah konsentrasi inflow (N organik, P sedimen/organik, NO3, P terlarut, klorofil-a, NH3, NO2, CBOD, oksigen terlarut) selama waktu tempuh, menghasilkan massa keluaran di `ht2`.
- **Masukan dan keluaran:** `ht1` (inflow: massa kg, air m3), `rc_prev` (titik rating untuk waktu tempuh), cuaca (`tave`, `solrad` MJ/m2, panjang hari jam); keluaran field nutrien/alga/CBOD/DO dan `temp` di `ht2`; `Channels::gra` (laju tumbuh alga tersimpan).
- **Cara kerja:**

1. Koefisien suhu tetap: θ tumbuh/respirasi alga 1,047; `rs1` 1,024; `rs2`, `rs3` 1,074; `rs4`, `rs5` 1,024; `bc2`, `bc3`, `bc4` 1,047 (`bc1` 1,083 dideklarasikan tetapi tak dipakai); `rk1` 1,047; `rk2`, `rk3` 1,024; `rk4` 1,060.
2. Waktu tinggal `τ = min(1, TT/24)` hari dari `rc_prev`; selang reaksi `Δt = 1` hari.
3. Konsentrasi inflow `c = 1000·M/Q_in` (mg/L) untuk sembilan spesies. Suhu air `T_w = 5 + 0,75·T_ave` (≤ 0 menjadi 0,1); `ht2.temp = T_w`.
4. Sumber bentik per luas dasar `chw·chl`: `rs2_s`, `rs3_s`, `rk4_s` (laju dikoreksi suhu dikali luas). Kedalaman `d` dari kurva rating pada debit inflow.
5. Bila `Q_in > 0` dan `d > 0` langkah 6–16 dijalankan; selain itu `ht2` dan tampungan saluran (`ch_stor`) di-nol-kan.
6. NH3 inflow ditambah sumber bentik `ammoin = c_NH3 + rs3_s/(1000·Q_in)`. Nitrogen anorganik untuk pembatasan pertumbuhan diambil dari **tampungan** saluran: `cinn = ch_stor.nh3 + ch_stor.no3`.
7. Oksigen jenuh (APHA) pada `T_K = T_w + 273,15`:

```latex
O_{sat} = \exp\left(-139.3441 + \frac{1.575701\times10^5}{T_K} - \frac{6.642308\times10^7}{T_K^2} + \frac{1.2438\times10^{10}}{T_K^3} - \frac{8.621949\times10^{11}}{T_K^4}\right)
```

bernilai 0 bila < 1e-6. DO inflow dibatasi pada [0,001; 30] mg/L.

8. Biomassa alga `A = 1000·chla/ai0`. Koefisien peneduhan, faktor nutrien, dan faktor cahaya (Steele, dirata-rata terhadap kedalaman):

```latex
\lambda = \min\left(\lambda_0,\ \lambda_0 + \lambda_1 a_0 A + \lambda_2 (a_0 A)^{0.66667}\right),\quad F_N = \frac{c_N}{c_N + K_N},\quad F_P = \frac{c_{solP}}{c_{solP} + K_P}
```

```latex
I_{alg} = \frac{H_{day}\,f_{act}}{T_L},\quad FL_1 = \frac{1}{\lambda d}\ln\frac{K_L + I_{alg}}{K_L + I_{alg}e^{-\lambda d}},\quad FL = 0.92\,\frac{T_L}{24}\,FL_1
```

(bila panjang hari `T_L` = 0, `I_alg = 1e-5`).

9. Laju tumbuh alga bila `A < 5000`: `igropt` 1 perkalian `μ·FL·F_N·F_P`; 2 minimum `μ·FL·min(F_N, F_P)`; 3 harmonik `μ·FL·2/(1/F_N + 1/F_P)` (0 bila salah satu ≤ 1e-6). Bila `A ≥ 5000`, laju lama dipakai (variabel *save* `gra`, bisa milik saluran lain).
10. Alga: `k = θ(gra) − θ(rhoq)`; `alg_m = wq_semianalyt(τ, 1, 0, k, A, A_in)` (dihitung dua kali identik, sehingga `alg_m2 = 0`); pengendapan `alg_set = θ(rs1)/d` (bila d > 0,001); `A_out = wq_semianalyt(τ, 1, alg_m, −alg_set, A, A_in)`, dibatasi [0, 5000] (nilai < 1e-6 menjadi 0); `chla = A_out·ai0/1000`.
11. CBOD: `m_1 = wq_k2m(τ, 1, −θ(rk1), c_B, c_B)`, `k_3 = −θ(rk3)/d`, `c_B = wq_semianalyt(τ, 1, m_1, k_3, c_B, c_B)`.
12. N organik: `bc3_k = 2·θ(bc3)`, `rs4_k = θ(rs4)/d`, `m_3 = wq_k2m(τ, 1, −bc3_k, c_ON, c_ON)`, `c_ON = wq_semianalyt(τ, 1, m_3, −rs4_k, c_ON, c_ON)` (≥ 0).
13. Oksigen: `rk2_m = θ(rk2)·O_sat`, `k = −θ(rk2)`; `bc1_m = wq_k2m(τ, 1, −θ(rk2), c_NH3, ammoin)`; `bc2_m = wq_k2m(τ, 1, −θ(bc2), c_NO2, c_NO2)`; suku sumber dan solusinya:

```latex
m_{DO} = m_1 + rk2_m - rs4_k + a_5\,bc1_m + a_6\,bc2_m,\qquad c_{DO} = \mathrm{semianalyt}(\tau, 1, m_{DO}, -\theta(rk_2), c_{DO}, c_{DO}) \ge 0
```

14. Rantai nitrogen (tanpa suku orde satu, k = 0): `c_NH3 = semianalyt(m = bc1_m − m_3; c_NH3, ammoin)`; `c_NO2 = semianalyt(m = −bc1_m + bc2_m)`; `c_NO3 = semianalyt(m = −bc2_m)`; masing-masing < 1e-6 menjadi 0.
15. Fosfor: `m_4 = wq_k2m(τ, 1, −θ(bc4), c_OP, c_OP)`, `rs5_k = θ(rs5)/d`, `c_OP = semianalyt(m_4, −rs5_k)`; P terlarut diperbarui eksplisit:

```latex
c_{solP} \mathrel{+}= \left(\theta(bc_4)\,c_{OP} + \frac{\theta(rs_2)}{chd_{db}} - a_2\,\theta(gra)\,A_{in}\right)\tau
```

16. Massa keluaran: `M_out = c·Q_in/1000` untuk kesembilan spesies.

- **Kebiasaan SWAT+ / catatan port:** semua dipertahankan apa adanya: (a) massa keluaran memakai volume **inflow**, bukan outflow hasil routing; (b) pembatasan N memakai massa tampungan (kg) terhadap `k_n` (mg/L); (c) λ tidak pernah melebihi λ0, sehingga peneduhan sendiri oleh alga tidak berpengaruh; (d) `alg_m`, sebuah konsentrasi, dipakai sebagai suku sumber; (e) laju hidrolisis N organik dikali 2; (f) `bc1_m` (NH3 → NO2) memakai laju reaerasi `rk2`, bukan `bc1`; (g) persamaan oksigen memakai suku `−rs4_k`; (h) beberapa variabel antara (`disoxin`, `dispin`, `cordo`, `f1`, `alg_m_o2`) dihitung tetapi tidak dipakai; (i) P terlarut memakai kedalaman `chd` baris data bernomor sama dengan saluran; (j) waktu tempuh dari `ht3%flo` yang tak pernah diisi, yaitu kurva pada debit nol (dikoreksi dengan `--fix-swat` menjadi debit inflow); (k) laju sudah dibagi `step` di `ChNut::new` tetapi fungsi ini berjalan sekali sehari.

### 12.9  Suhu air sungai (`channel.rs`)

#### 12.9.1  `Channels::ch_temp`

- **Lokasi:** `crates/swat-engine/src/channel.rs`, `fn ch_temp(&mut self, ich: usize, hd1: Hyd, ht2: Hyd, ins: &ChanInflows, cx: &ChanCtx) -> f32`.
- **Padanan SWAT+:** `ch_temp.f90` (model suhu air dengan kontribusi aliran lahan bersuhu udara tertunda dan pertukaran panas menuju suhu kesetimbangan).
- **Tujuan:** menghitung suhu air saluran (°C) dari suhu air hulu, kontribusi air lokal dari RU (salju leleh, limpasan, aliran lateral) dengan suhu udara tertunda, dan pertukaran panas permukaan selama waktu tempuh.
- **Masukan dan keluaran:** `hd1` (outflow), `ht2` (outflow hasil routing), inflow (`ins`), cuaca (`tave`, `dewpt`, `windsp` m/s, `solrad`, `solradmx` MJ/m2, `rhum`), pengukur suhu stasiun (`tgage`), `hru_wb` (mm). Keluaran: suhu akhir; menyetel `ch_stor.temp`, `Channels::wtemp`, `SdCh::wtemp`.
- **Cara kerja:**

1. **Suhu hulu.** Mode asli: untuk setiap inflow secara berurutan, bila berasal dari saluran SWAT-DEG, suhu = rata-rata tertimbang debit dari inflow itu dan inflow **sebelumnya** (inflow pertama dipasangkan dengan hidrograf nol, karena Fortran membaca `hin_d(0)` di luar batas); bila bukan dari saluran, `5 + 0,75·T_ave`. Hanya inflow terakhir yang menentukan. Mode `--fix-swat`: rata-rata tertimbang debit semua inflow dari saluran SWAT-DEG (atau `5 + 0,75·T_ave` bila debitnya < 1e-6 m3/s).
2. **Air lokal dari RU.** Untuk setiap RU yang menjadi inflow dan setiap elemennya (HRU), bila baris tabel elemen global bernomor **penghitung loop** bertipe `hru`: `q_sno += sno·frac`, `q_surf += surq`, `q_lat += latq`, `q_wyld += wyld` (mm; hanya salju yang dikali fraksi). `q_wyld = 0` menjadi 1e-6; `q_gw = 0`.
3. **Suhu lokal.** Tanpa pengukur suhu: `T_loc = 5 + 0,75·T_ave`. Dengan pengukur: suhu udara rata-rata `(T_max + T_min)/2` selama jeda `lag` hari (salju 2, permukaan 5, lateral 10, airtanah 250; pada tahun simulasi pertama `lag = min(jday, lag_0)`; bila jendela melewati awal tahun dan sudah lebih dari satu tahun data, hari-hari akhir tahun sebelumnya dipakai; jumlahnya dibagi `lag`):

```latex
T_{loc} = \frac{0.75\,T_{sno}\,q_{sno} + 0.75\,T_{gw}\,q_{gw} + 0.75\,T_{lat}\,q_{lat} + 0.75\,T_{surf}\,q_{surf}}{q_{wyld}}
```

dengan `T_surf, T_lat, T_gw ≥ 0,01` dan `T_sno` dalam [0,01; 2,5]. Bila `|T_loc − T_out,kemarin| > 5`, `T_loc = 5 + 0,75·T_ave` (`T_out,kemarin` = suhu `ch_out` hari sebelumnya).

4. **Pencampuran** dengan `Q_up` debit `hd1` (m3/s):

```latex
T_{init} = \frac{T_{up}\,Q_{up} + T_{loc}\,q_{wyld}}{Q_{up} + q_{wyld}}\quad (Q_{up} > 10^{-6}),\qquad T_{init} = T_{loc}\ \text{selain itu}
```

5. **Suhu kesetimbangan.** `T_dx = min(0,75·T_dew, T_air)`, `T_md = (T_init + T_dx)/2`:

```latex
\beta = 0.35 + 0.015\,T_{md} + 0.0012\,T_{md}^2,\quad f(u) = 9.2 + 0.46\,u^2,\quad K_e = 4.48 + 0.05\,T_{init} + (\beta + 0.47)\,f(u)
```

```latex
H_{sr} = 0.97\,(11.57\,R_s)(1 - 0.5),\quad e_s = 6.1275\,e^{17.62T/(237.3+T)},\quad e_a = e_s\,RH,\quad C = 0.9\frac{R_s}{R_{s,max}} + 0.1
```

```latex
\varepsilon_a = 0.74 + 0.0065\,e_a\,(1 + 0.17C^2),\quad H_{atm} = 0.96\,\varepsilon_a\,\sigma\,T_K^4,\quad T_{eq} = T_{dx} + \frac{H_{sr}}{K_e} + \frac{H_{atm} - 305.5 - 4.48\,T_{dx}}{K_e}
```

(`C = 0` bila `R_s,max < 1e-4`; σ = 5,67e-8 W/(m2·K4); faktor 11,57 mengubah MJ/m2/hari ke W/m2; `ssff` = 0,5 fraksi naungan).

6. **Kedalaman aliran.** `v_c = max(0,01, vel)`; lebar dan kedalaman dari `db_chw`, `db_chd` (baris bernomor saluran), `hex_coef2 = 1,5`:

```latex
W_f = \sqrt{\frac{2\,Q_{out}/v_c}{(chd/1.5)/chw}},\qquad D_f = \min\left(2\,chd,\ \max\left(0.1,\ \frac{chd/1.5}{chw}\,W_f\right)\right)
```

(bila `chw < 0,1`, `chw` diganti `0,25·chw` **setelah** `W_f` dihitung).

7. **Pertukaran panas** selama waktu tempuh `rttime/24` hari:

```latex
\Delta T = \frac{K_e\,(T_{eq} - T_{init})}{1000\cdot 4186\,D_f/86400}\cdot\frac{TT}{24},\qquad T_w = \max(0.001,\ T_{init} + \Delta T)
```

8. `ch_stor.temp = T_w`; `Channels::wtemp = 5 + 0,75·T_ave`; `SdCh::wtemp` = `T_w` (koreksi) atau `wtemp` (asli). Kembalikan `T_w`.

- **Kebiasaan SWAT+ / catatan port:** (a) suhu hulu dari inflow terakhir dan inflow sebelumnya saja, termasuk pembacaan di luar batas untuk inflow pertama (asli; dikoreksi dengan `--fix-swat`); (b) elemen RU disaring dengan indeks penghitung loop ke tabel elemen global, bukan indeks elemen sebenarnya; (c) limpasan dan aliran lateral tidak dikali fraksi elemen, hanya salju; (d) `T_up·Q_up` (m3/s) dicampur dengan `q_wyld` (mm) seperti di Fortran; (e) koefisien default `w_temp(0)` dipakai karena `temperature.cha` tidak didukung (proyek dengan file itu ditolak `Channels::new`); (f) `vel` dan `rttime` adalah milik hari terakhir yang ada inflow; (g) pada mode asli hasilnya hanya mencapai `ch_stor.temp`, dan `channel_sd_day` mencetak satu nilai `wtemp` untuk semua saluran (di Bh_land, koreksi memberi 21–28 °C berbeda per saluran pada satu hari pertengahan tahun).

### 12.10  Inventaris fungsi Bab 12

Semua 35 fungsi di kedua file (hasil `grep -n "fn "`), dengan subbab tempat uraiannya:

| Fungsi | File | Subbab |
|---|---|---|
| `AquDay::columns` | route.rs | 12.4.3 (Bab 14) |
| `AquDay::columns_as` | route.rs | 12.4.3 (Bab 14) |
| `Network::new` | route.rs | 12.1.1 |
| `Network::hyd_flo_day1` | route.rs | 12.2.6 |
| `Network::next_day` | route.rs | 12.2.4 |
| `Network::flow_hyd` | route.rs | 12.2.2 |
| `Network::shift` | route.rs | 12.2.3 |
| `Network::hru_order` | route.rs | 12.2.6 |
| `Network::hru_runon` | route.rs | 12.3.1 |
| `Network::set_hru` | route.rs | 12.3.2 |
| `Network::day` | route.rs | 12.4.1 |
| `Network::sum_tsin` | route.rs | 12.2.5 |
| `Network::ru_control` | route.rs | 12.4.2 |
| `Network::aqu_1d_control` | route.rs | 12.4.3 (Bab 14) |
| `Network::aquifers` | route.rs | 12.4.3 (Bab 14) |
| `Network::channels` | route.rs | 12.4.4 |
| `Network::reservoir_days` | route.rs | 12.4.4 |
| `Network::recall_days` | route.rs | 12.4.4 |
| `Network::channel_steps` | route.rs | 12.4.4 |
| `Network::routing_units` | route.rs | 12.4.4 |
| `unit_hyd` | route.rs | 12.2.1 |
| `Rc::scale` | channel.rs | 12.5.1 |
| `Rc::interp` | channel.rs | 12.5.2 |
| `qman` | channel.rs | 12.5.3 |
| `theta` | channel.rs | 12.5.4 |
| `wq_semianalyt` | channel.rs | 12.5.5 |
| `wq_k2m` | channel.rs | 12.5.6 |
| `ChNut::new` | channel.rs | 12.5.7 |
| `Channels::new` | channel.rs | 12.6.1 |
| `Channels::interp_flo` | channel.rs | 12.6.2 |
| `Channels::output` | channel.rs | 12.6.3 |
| `Channels::wtemp_of` | channel.rs | 12.6.3 |
| `Channels::control` | channel.rs | 12.7.1 |
| `Channels::ch_watqual4` | channel.rs | 12.8.1 |
| `Channels::ch_temp` | channel.rs | 12.9.1 |

Ringkasan koreksi `--fix-swat` dalam bab ini (dari `docs/swat-fixes.md`): `ru_d` hanya tot + rhg; trapesium benar pada kurva rating dan waktu tampung Muskingum; lebar atas `chw` di atas bankfull; waktu tempuh QUAL2E pada debit inflow; outflow membawa suhu sungai yang dihitung; suhu hulu tertimbang debit semua saluran hulu; suhu per saluran di `channel_sd_day`.
