## Glosarium

Glosarium ini menghimpun istilah teknis dari Bab 3–17: nama file dan variabel SWAT+, konsep hidrologi dan agronomi, serta istilah perangkat lunak port Rust. Kolom Bab menunjukkan bab tempat istilah itu dibahas.

### A

| Istilah | Definisi | Bab |
| --- | --- | --- |
| ab\_emer | Opsi pelepasan seluruh volume di atas pelimpah darurat. | 13 |
| ABI (Application Binary Interface) | Kontrak tingkat biner antara modul dan pemanggil (nama fungsi, tipe argumen, tata letak memori); modul swat-wasm memakai ABI C mentah berupa pointer dan panjang. | 16 |
| --abs / --rel | Toleransi mutlak (bawaan 0,001) dan relatif (bawaan 1e-4) perintah compare. | 16 |
| absmin / absmax | Batas bawah dan atas nilai parameter setelah perubahan. | 14 |
| Abstraksi awal urban (`urb_init_abst`, `urb_abstinit`) | Tampungan awal di area kedap air yang harus terisi sebelum limpasan terjadi (mm). | 11 |
| absval / abschg / pctchg / relchg | Jenis perubahan parameter kalibrasi: ganti dengan nilai mutlak, tambah nilai mutlak, ubah persen, dan ubah relatif (faktor 1 + v). | 4, 14 |
| adj\_pkr (`parameters.bsn`) | Faktor penyesuaian laju hujan puncak. | 6 |
| Air tersimpan (`st`, `sw`) | Air lapisan di atas titik layu; `sw` jumlah seluruh profil (mm). | 7 |
| airlag\_temp / tlag | Suhu udara rerata enam hari sebelumnya, dari larik melingkar enam hari. | 6 |
| Aksi (`act`, `Action`) | Tindakan tabel keputusan (irigasi, pupuk, olah tanah, tanam, panen, mati, tinggi pelimpah, pelumpuran, reset rotasi). | 10 |
| Akuifer 2-D (`aqu2d`) | Varian akuifer SWAT+ yang membagi baseflow ke kanal menurut panjang kontribusi; tidak diport (ditolak). | 14 |
| Akuifer dangkal (1-D) (`aqu`) | Objek SWAT+ yang menyimpan air tanah sebagai satu tampungan (mm) dan melepaskan aliran balik (aliran dasar), revap, serta rembesan ke akuifer dalam; disimulasikan oleh `aqu_1d_control`. | 3, 14 |
| Albedo (`albday`) | Pantulan radiasi permukaan hari ini. | 7 |
| Aliran balik (baseflow, return flow, `flo`) | Air yang keluar dari akuifer dangkal ke kanal, waduk atau objek lain, dihitung dengan resesi eksponensial. | 14 |
| Aliran lateral (`latq`, `flat`) | Aliran bawah permukaan menuruni lereng dari lapisan di atas kapasitas lapang (mm). | 7 |
| Aliran terkonsentrasi (`con_vfs`, `cha_q`) | Fraksi lahan yang mengalir ke 10 % jalur, dan fraksi aliran itu yang tersalurkan penuh tanpa tersaring. | 11 |
| Alternatif (`alt`, `alts`, `act_hit`) | Kolom tabel keputusan. Kondisi digabung DAN di dalam satu alternatif dan antaralternatif digabung ATAU, sehingga alternatif berlaku bila semua kondisinya terpenuhi. | 10, 13 |
| Ambang (weir, `weir.res`, `weir_hgt`) | Struktur penahan genangan sawah; tingginya (`weir_hgt`, m) menentukan volume utama dan dapat diubah aksi `weir_height`. SWAT+ selalu memakai baris pertama berkas `weir.res` untuk lahan basah. | 10, 11, 17 |
| amp\_r | Rerata fraksi hujan setengah jam per bulan, dari hujan setengah jam maksimum dan `adj_pkr`. | 6 |
| apply\_day | Hari yang terundi oleh `prob_unif`. | 10 |
| aqu\_catunit.ele | Berkas elemen wilayah akuifer; keberadaannya mengaktifkan keluaran akuifer basin. | 14 |
| aqu\_hdr | Header tabel akuifer SWAT+ yang memberi label `orgn`/`orgp` pada kolom karbon dan N organik (kebiasaan SWAT+). | 15 |
| AquDay (`aquifer_dynamic`, `aqu_d`) | Status harian akuifer: 17 kolom keadaan dan fluks (aliran, kedalaman muka air, simpanan, imbuhan, rembesan, revap, nitrat, P mineral, karbon, dst.) yang juga menjadi baris `aquifer_day.txt`. | 5, 14, 15 |
| aquifer\_database (`aqudb`, `AquiferDb`) | Satu baris `aquifer.aqu` berisi parameter dan kondisi awal akuifer. | 14 |
| Area kedap air (`fimp`) | Fraksi HRU perkotaan yang tidak dapat meresapkan air. | 7, 11 |
| Area kedap air terhubung langsung (`fcimp`) | Bagian area kedap air yang mengalirkan limpasannya langsung ke saluran. | 7, 11 |

### B

| Istilah | Definisi | Bab |
| --- | --- | --- |
| backspace | Perintah Fortran untuk mundur satu record agar baris dibaca ulang; ditiru port dengan `Mark`/`reset`. | 4 |
| Bankfull | Kondisi air setinggi tebing alur; di atasnya air melimpah ke dataran banjir. | 12 |
| basin\_crop\_yld | Luas panen (ha), hasil (t), dan t/ha setiap tanaman di tingkat cekungan per tahun dan rata-rata tahunan. | 5 |
| basin\_prm\_default | Subrutin SWAT+ yang mengganti parameter basin bernilai nol atau kosong (< 1e-6) dengan nilai bawaan operasional; ditiru port oleh `BasinParams::complete`. | 3, 4 |
| basin\_sw\_init | Rutin yang menetapkan air tanah dan salju awal pada tahun tercetak pertama. | 5 |
| Benih (`rndseed`, seed) | Bilangan bulat keadaan generator bilangan acak; setiap generator cuaca punya sepuluh. | 6 |
| Berat isi tanah (bulk density, `bd`, `avbd`) | Kerapatan isi tanah (g/cm3); `avbd` adalah rata-rata profil. | 7, 14 |
| Bh\_land | Batanghari dengan PET Hargreaves dan HRU perkotaan/lahan basah diganti hutan, untuk validasi fase lahan dan routing. | 17 |
| Bh\_res / Bh\_rel / Bh\_nosed | Skenario validasi Batanghari dengan reservoir hipotetis dan dua sumber titik (recall); Bh\_nosed (1.0.1) memakai baris sediment.res yang tidak ada; Bh\_rel memakai opsi pelepasan baru dari tabel `lake_multi`. | 13, 17 |
| Bilangan kurva (curve number, CN, `cn2`, `cnday`) | Parameter SCS yang menyatakan potensi limpasan. `cn2` adalah nilai pada kondisi kelembapan II dan `cnday` nilai harian, yang pada Green dan Ampt diperbarui per langkah. | 7, 11, 14 |
| Biomassa biozona (`biom`, `rbiom`) | Massa bakteri hidup dalam biozona dan perubahan hariannya (kg/ha). | 11 |
| Biomassa mikroba (BM, `microb`) | Kolam karbon aktif mikroba tanah. | 9 |
| Biomassa minimum (`bm_min`) | Biomassa (atau residu) minimum, kg/ha, yang harus tersisa agar panen dilakukan. | 8, 10 |
| Biozona (biozone, `sep_biozone`) | Lapisan tanah di bawah bidang resapan septik yang dihuni biomassa bakteri dan mengolah efluen; biomassa dan plaknya mengubah sifat hidraulik tanah, dan proses N/P-nya ditangani modul septik. | 9, 11, 17 |
| Bit demi bit (bit-for-bit) | Tujuan port: keluaran identik secara biner dengan SWAT+ Fortran sampai bit terakhir setiap angka yang dicetak (f32), termasuk urutan operasinya, tanpa memakai toleransi. | 4, 5, 11, 12, 16, 17 |
| black\_box (`std::hint::black_box`) | Fungsi Rust yang menyembunyikan nilai dari optimizer agar panggilan `powf` tidak diganti `sqrt`. | 5 |
| `bm_dieoff` | Pengali kematian biomassa berlebih di `pl_mortality`. | 8 |
| bm\_max | Biomassa maksimum periode (kg/ha). | 5 |
| BMP (best management practice) | Praktik konservasi untuk mengurangi polutan; di bab ini jalur filter, saluran berumput, dan efisiensi pengguna. | 11 |
| `bmx_peren` | Biomassa maksimum perennial (t/ha); kelebihannya mati di akhir tahun. | 8 |
| BOD (biochemical oxygen demand, `bio_bod`) | Kebutuhan oksigen biokimia, ukuran bahan organik terurai dalam efluen. | 11 |
| Box–Muller (`cli_dstn1`) | Transformasi dua bilangan seragam menjadi satu deviasi normal baku. | 5 |
| bsn\_frac / ru\_frac | Bagian luas HRU terhadap cekungan / unit lanskapnya, dipakai sebagai bobot neraca wilayah. | 5 |
| `bss` | Simpanan lag aliran lateral (mm); `bss_tile` untuk aliran tile. | 7 |

### C

| Istilah | Definisi | Bab |
| --- | --- | --- |
| c\_microb\_sed | Karbon mikroba yang hilang bersama sedimen; variabel bersama yang dipertahankan antarpanggilan. | 9 |
| c\_ratio | Rasio kapasitas tampungan terhadap inflow tahunan rata-rata (Hanasaki). | 13 |
| cal\_group | Label grup kalibrasi pada tata guna lahan HRU. | 14 |
| cal\_parm\_select | Subrutin SWAT+ yang memilih field menurut nama parameter dan menghitung ulang turunannya. | 14 |
| cal\_parms.cal (`CalParm`) | Berkas daftar parameter yang dapat dikalibrasi, jenis objeknya dan batas mutlaknya. | 14 |
| calibration.cal (`CalUpdate`) | Berkas daftar perubahan parameter dengan jenis perubahan, nilai, kondisi, lapisan dan objek. | 14 |
| CBOD | Kebutuhan oksigen biokimia karbon (mg/L) pada limpasan dan air saluran; di kanal meluruh orde satu dan mengendap. | 7, 12 |
| cbod / dox | Kebutuhan oksigen biokimia karbon dan oksigen terlarut (kg), tidak diproses di reservoir. | 13 |
| cdn (β\_denit) | Koefisien laju denitrifikasi. | 9 |
| cdylib | Jenis crate Rust yang menghasilkan pustaka dinamis/modul dengan ABI C, dipakai untuk `.wasm`. | 16 |
| Celeritas (`celerity_bf`) | Kecepatan rambat gelombang banjir di saluran, diambil 5/3 kecepatan aliran menurut Manning. | 11, 12 |
| channel\_sd\_subday.txt | Tabel aliran keluar kanal SWAT-DEG per langkah waktu sub-harian. | 15 |
| check-parquet.py | Skrip CI yang memeriksa tabel Parquet dengan pyarrow dan DuckDB terhadap tabel teksnya. | 16, 17 |
| Checksum tar | Jumlah bita header dengan field checksum dianggap 8 spasi, ditulis oktal. | 15 |
| chem\_app.ops (`surf_frac`) | Metode aplikasi pupuk; fraksi yang tinggal di lapisan permukaan. | 10 |
| chg\_par | Fungsi SWAT+ yang menghitung nilai parameter baru dan membatasinya. | 14 |
| chla | Klorofil-a (kg); di reservoir SWAT+ ini hanya nilai semu (V + Q)·10^−6. | 13 |
| CHUNK | Ambang 4 MiB penyangga per berkas sebelum dialirkan ke store. | 15 |
| ChWat (`water_body`, `res_wat_d`) | Neraca air permukaan harian badan air kanal atau reservoir: luas (ha), presipitasi, evaporasi, dan rembesan (m3). | 5, 13, 15 |
| cli\_bounds\_check | Pemeriksaan apakah hari simulasi di dalam rentang rekaman file terukur. | 6 |
| Clock (`time`) | Jam simulasi berisi tahun, hari, bulan, langkah waktu, dan tabel `ndays`. | 5 |
| cmn (β\_min) | Koefisien laju mineralisasi humus aktif (1/hari). | 9 |
| `cn3_swf` | Fraksi antara kapasitas lapang dan kejenuhan profil tempat CN mencapai nilai kondisi III (metode kelembapan tanah). | 7, 14 |
| `cn_froz` | Koefisien pengurang retensi pada tanah beku (bawaan 0,000862). | 7 |
| `cnyld`, `cpyld` | Fraksi N dan P dalam hasil/biji. | 8 |
| codes.bsn | File kode pilihan metode tingkat basin, misalnya metode PET (`pet`), model karbon (`cswat`), `nam1`, `rte`, dan hidrograf satuan. | 3, 4 |
| Column chunk | Data satu kolom dalam satu row group; di port berisi satu data page. | 15 |
| compare | Subperintah CLI `swatplus-rust compare` yang membandingkan tabel SWAT+ dan tabel port kolom demi kolom menurut kunci tahun, hari, unit, dan langkah, dengan toleransi `--abs`/`--rel`. | 15, 16, 17 |
| compare-yields.py | Skrip validasi pembanding tabel hasil panen (`crop_yld_aa`, `basin_crop_yld_*`) antara SWAT+ dan port. | 16, 17 |
| conc\_nmin / conc\_pmin | Konsentrasi minimum N dan P di bawahnya tidak ada pengendapan (mg/L). | 13 |
| const2 | Parameter kedua aksi; batas jumlah per tahun, dan untuk irigasi genangan juga kedalaman ambang sawah. | 10 |
| ConvertedType UTF8 | Anotasi Parquet yang menandai BYTE\_ARRAY sebagai teks UTF-8. | 15 |
| `covmx`, `cov50` | Kedalaman salju untuk tutupan penuh dan fraksinya untuk tutupan 50 %. | 7 |
| CRLF | Akhir baris gaya Windows (`\r\n`); dibaca sama dengan `\n`. | 4 |
| crop\_yld | Bendera `print.prt` untuk tabel hasil panen: `y` basin, `a` per HRU, `b` keduanya. | 15 |
| crop\_yld\_aa | Hasil rata-rata per panen setiap tanaman HRU selama simulasi (kg/ha massa, C, N, P). | 5 |
| cswat | Kode model karbon tanah di `codes.bsn`: 0 model asli, 2 model SWAT-C (Zhang) yang membaca `carbon.bsn` dan file lapisannya. | 4, 9, 10 |
| ctbl\_ | Awalan nama tabel kondisi alternatif (`res_rel_conds`), tidak didukung port. | 13 |
| curno | Subrutin yang menghitung ulang CN1, CN3 dan parameter retensi dari CN2. | 14 |

### D

| Istilah | Definisi | Bab |
| --- | --- | --- |
| d50 | Diameter butir median sedimen, µm. | 13 |
| data page / PageHeader | Halaman data Parquet beserta header Thrift-nya (tipe, ukuran, jumlah nilai, encoding). | 15 |
| Dataran banjir (`fp`, `fps`, `fpn`) | Bagian di luar alur dengan kemiringan dan n Manning sendiri; tampungan di atas volume bankfull dicatat sebagai `fp_stor`. | 12 |
| day\_end\_yr | Hari terakhir loop pada tahun berjalan (365/366, atau `day_end` pada tahun terakhir). | 5 |
| day\_lag\_mx | Jumlah hari maksimum penyimpanan hidrograf untuk penundaan; SWAT+ menetapkannya 2. | 3 |
| daylength | Panjang hari (jam) dari lintang dan deklinasi matahari. | 6 |
| daylmn | Panjang hari minimum tahunan di lintang stasiun (jam). | 6 |
| daylth (`dorm_hr`) | Ambang panjang hari (jam di atas minimum) yang memicu dormansi tanaman tahunan. | 6 |
| days | Opsi pengurasan volume di atas basis dalam sejumlah hari. | 13 |
| days\_plant/harv/kill/irr | Jumlah hari sejak tanam, panen, mati, irigasi terakhir; mulai −366. | 10 |
| days\_prt | Jumlah hari tercetak, pembagi kolom keadaan rata-rata tahunan. | 5 |
| dbout / use\_obj\_labels | Bendera `print.prt`; bila `n`, baris objek dibaca dalam urutan tetap tanpa melihat namanya. | 4 |
| Debit puncak (`qp_cms`) | Laju limpasan puncak harian (m3/s) dari metode rasional termodifikasi. | 7, 9 |
| decr\_min | Laju dekomposisi residu minimum harian. | 9 |
| define\_unit\_elements | Format dan subrutin ekspansi daftar objek ringkas, di mana bilangan negatif menutup rentang dari bilangan sebelumnya (`1 -4` → 1, 2, 3, 4). | 3, 4, 14 |
| definition/repetition level | Penanda null dan pengulangan pada kolom bersarang Parquet; nol untuk kolom required datar. | 15 |
| Defisit tekanan uap (VPD, `vpd`) | Selisih tekanan uap jenuh dan aktual udara (kPa); di atas 1 kPa menurunkan RUE. | 6, 7, 8 |
| Deklinasi matahari | Sudut posisi matahari terhadap ekuator, fungsi hari Julian. | 6 |
| Denitrifikasi (`wdn`, `denit`) | Reduksi nitrat menjadi gas N oleh mikroba pada tanah jenuh, dihitung bila faktor air melebihi `sdnco`; di biozona septik dihitung dengan laju orde satu sebanding biomassa. | 9, 11 |
| `dep_co` | Koefisien deposisi untuk kapasitas angkut sedimen run-on. | 7 |
| Deposisi atmosfer (`atmo.cli`, `nut_nrain`) | Masukan nitrat dan amonium ke tanah lewat hujan (konsentrasi mg/L) dan deposisi kering (kg/ha). | 3, 4, 5, 6 |
| Deviat normal baku | Bilangan acak berdistribusi N(0,1), di sini dari transformasi Box–Muller (`dstn1`). | 6 |
| dew\_is\_rhum | Penanda bahwa kolom titik embun generator berisi kelembapan relatif (semua nilai 0–1). | 6 |
| DirStore | Store yang menulis berkas tabel keluaran ke sebuah folder, membuat berkas pada penulisan pertama. | 15, 16 |
| Distribusi akar (`rtfr`) | Fraksi akar di setiap lapisan tanah, dari fungsi kerapatan 1,15·e^(−11,7x) + 0,022. | 8, 10 |
| Distribusi segitiga (`atri`) | Distribusi peluang dengan batas bawah, modus, dan batas atas; di SWAT+ diskalakan ke rata-rata dan dibatasi (0, 1), dipakai untuk kelembapan relatif dan fraksi hujan setengah jam. | 5, 6 |
| Distribusi serapan (`n_updis`, `p_updis`, `water_dis`) | Parameter eksponensial yang menentukan seberapa terpusat serapan di dekat permukaan. | 8 |
| Distribusi skew-normal | Transformasi deviat normal dengan koefisien kemencengan (`pcp_skew`) untuk jumlah hujan harian. | 6 |
| Diversi | Aliran negatif pada recall (pengambilan air), tidak didukung port. | 13 |
| Dormansi (`idorm`, `dormhr`, `daylmn`) | Masa istirahat tanaman saat panjang hari di bawah minimum tahunan ditambah ambang. | 8 |
| Drainase bawah permukaan (tile drain, `qtile`, `ldrain`) | Drainase bawah tanah buatan melalui pipa; `qtile` aliran ke pipa, `ldrain` indeks lapisannya, dan `drain_co` batas harian (mm). | 7, 9 |
| dyrt / dyrt1 | Opsi pengurasan dalam sejumlah hari ditambah laju dasar persentase volume utama; `dyrt1` mengambil basis (target volume) dari kondisi tabel (con1/con2/con3 = kondisi ke-5/4/3). | 13, 17 |

### E

| Istilah | Definisi | Bab |
| --- | --- | --- |
| Efisiensi BMP pengguna (`bmpuser.str`) | Fraksi tetap (0–1) sedimen dan hara yang dibuang oleh praktik buatan pengguna. | 11 |
| Efisiensi panen (`eff`) | Fraksi bagian yang dipanen yang benar-benar terangkut; sisanya kembali ke residu atau humus. | 10 |
| Efisiensi penggunaan radiasi (RUE, `bio_e`) | Biomassa yang dihasilkan per satuan radiasi aktif fotosintesis yang diintersepsi ((kg/ha)/(MJ/m²)). | 8 |
| Efisiensi tangkap dataran banjir (`trap_eff`) | Bagian sedimen dan nutrien terikat yang mengendap saat debit puncak melebihi 1,5 kali bankfull. | 12 |
| Efluen tangki septik (`qstemm`) | Aliran air limbah dari tangki ke biozona; di SWAT+ 6f51536 selalu nol. | 11 |
| EI (`usle_ei`) | Indeks erosivitas hujan (energi × intensitas 30 menit maksimum). | 9 |
| Eksklusi anion (`anion_excl`, θe) | Fraksi porositas yang tidak dapat dimasuki anion seperti nitrat; parameter profil tanah. | 9, 14 |
| Elemen RU (`ru_elem`) | Satu anggota RU (di port hanya HRU) beserta fraksi dan delivery ratio-nya. | 12 |
| Energi kinetik hujan (KE) | Energi tumbukan hujan langsung dan tetesan daun (J/m2/mm) yang mendorong erosi percikan. | 11 |
| env.swat\_progress | Fungsi impor yang wajib disediakan pemanggil; dipanggil dengan (hari selesai, total hari) setiap persen bulat. | 16 |
| `epco` | Faktor kompensasi serapan air tanaman, yang mengizinkan lapisan lebih dalam menutup kekurangan serapan lapisan atas. | 7, 8, 14 |
| EPIC | Erosion-Productivity Impact Calculator, model asal persamaan pertumbuhan tanaman SWAT. | 8 |
| erorgn / erorgp | Enrichment ratio masukan pengguna untuk N dan P organik. | 9 |
| Erosi percikan (splash erosion) | Pelepasan partikel tanah oleh tumbukan butir hujan; di SWAT+ diadopsi dari EUROSEM. | 11 |
| Erosi tebing (`ebank`) | Mundurnya tebing bila kecepatan di tikungan melampaui kecepatan kritis yang bergantung kohesi, vegetasi, dan kerapatan tanah. | 12 |
| `es_day` | Evaporasi tanah aktual (mm). | 7 |
| `esco` | Faktor kompensasi evaporasi tanah, yang mengatur seberapa dalam kebutuhan evaporasi dapat diambil. | 7, 14 |
| `etco` | Batas 80 % air lapisan yang dapat diuapkan per hari. | 7 |
| Evapotranspirasi potensial (PET) | Laju evapotranspirasi maksimum dari permukaan bervegetasi acuan tanpa batasan air (mm/hari). Metodenya Priestley-Taylor, Penman-Monteith (`pet` = 1), Hargreaves (`pet` = 3), atau dibaca; revap akuifer memakai PET stasiun cuaca. | 3, 5, 6, 7, 13, 14 |
| Event JSON (start, progress, note, table, done, error) | Objek JSON satu baris yang dipancarkan runner ke stdout dan dibaca RustGIS untuk memantau jalannya engine. | 15, 16 |
| evrch | Koefisien penguapan permukaan air saluran terhadap PET. | 12 |
| Ew.d / e15.4 | Deskriptor format Fortran untuk notasi eksponensial dengan mantisa `0.dddd` dan eksponen dua digit, lebar 15. | 15 |
| exp\_w | Pembungkus eksponensial aman yang menjepit argumen ke \[−80, 80\]. | 9 |

### F

| Istilah | Definisi | Bab |
| --- | --- | --- |
| f12.3 | Deskriptor format Fortran titik tetap lebar 12 dengan tiga desimal; dipakai tabel HRU, LSU, basin. | 15 |
| Faktor C (`usle_cfac`) | Faktor penutupan dan pengelolaan USLE, dihitung dari residu dan biomassa. | 7, 9 |
| Faktor koreksi suhu (`theta`) | Koreksi laju reaksi dari 20 °C ke suhu lain, k\_T = k\_20·θ^(T−20) (QUAL2E III-52). | 12, 13 |
| Faktor leleh (`meltmx`, `meltmn`) | Laju leleh per derajat pada 21 Juni dan 21 Desember (mm/°C/hari). | 7 |
| Faktor LS | Faktor topografi panjang dan kemiringan lereng USLE. | 7 |
| Faktor pengatur (`reg`) | Stres terkecil di antara keenam stres; pengali pertumbuhan biomassa. | 8 |
| Faktor pengulangan (`r*c`, `r*`) | Notasi list-directed untuk mengulang nilai `c` sebanyak `r` kali, atau `r` nilai null. | 4 |
| Faktor siklus nutrien (`csf`, γ\_c) | Akar hasil kali faktor air (`sut`) dan faktor suhu (`cdg`) yang mengatur laju dekomposisi. | 9 |
| `fcgd` | Faktor suhu kurva beta untuk proses biologis. | 7 |
| fertilizer.frt | Basis data komposisi pupuk (`fminn`, `fminp`, `forgn`, `forgp`, `fnh3n`). | 10 |
| fertn, fertp | N dan P pupuk total yang diberikan pada hari itu (kg/ha). | 10 |
| `ffc` / `ffcb` | Fraksi kapasitas lapang yang menjadi air tanah awal. | 7, 14 |
| Field header (Thrift) | Bita `(delta << 4) \| tipe` bila selisih nomor field 1–15, atau tipe + varint zigzag nomor field. | 15 |
| Fiksasi nitrogen (`fixn`, `nfix_co`, `nfixmx`) | N yang diikat legum; `nfix_co` mencampur model SWAT dan pemenuhan penuh, `nfixmx` batas harian. | 8 |
| File koneksi (`*.con`) | File atribut objek spasial dan aliran keluarnya. | 4 |
| file.cio | File induk (daftar isi) skenario SWAT+ yang menamai file masukan untuk setiap slot; dibaca menurut posisi baris, bukan nama baris, dan judulnya dipakai di baris pertama tabel keluaran. | 3, 4, 16 |
| FileMetaData | Metadata akhir berkas Parquet (skema, jumlah baris, row group, created\_by). | 15 |
| `--fix-swat` | Opsi `run` port yang mengoreksi bug SWAT+ terdokumentasi (`docs/swat-fixes.md`) alih-alih mereproduksinya, misalnya label kolom akuifer menjadi `cbn`/`orgn`. Tanpa opsi ini port meniru SWAT+ bit demi bit; dengan opsi ini hasil berbeda dari SWAT+. | 7, 8, 10, 11, 12, 14, 15, 17 |
| flags | Bit mask argumen `swat_run`: 1 Parquet, 2 teks, 4 koreksi bug SWAT+. | 16 |
| flo\_cha / flo\_res / flo\_ls (mm) | Bagian aliran balik yang menuju kanal, waduk, atau objek lanskap/akuifer lain. | 14 |
| flo\_min (m) | Kedalaman muka air maksimum agar aliran balik terjadi. | 14 |
| Fluks vs keadaan | Kolom yang dijumlah (mm, kg/ha per periode) vs kolom yang dirata-rata (simpanan, CN, LAI, suhu). | 5 |
| fortran-bin | Cabang git repo tempat biner statis SWAT+ Fortran acuan (commit 6f51536, hasil alur kerja `fortran.yml`) disimpan untuk validasi. | 16, 17 |
| fortran\_eq | Perbandingan string ala Fortran yang mengabaikan spasi di ujung kanan. | 3 |
| Fotodegradasi (`photo_degrade_factor`) | Kehilangan harian residu permukaan oleh sinar matahari. | 9 |
| frac\_seq | Fraksi karbon organik awal yang dialokasikan ke kolam humus dan mikroba. | 9 |
| Fraksi akar (`root_frac`, `rsr1`, `rsr2`) | Fraksi biomassa total di akar, menurun dari `rsr1` di awal ke `rsr2` di akhir musim. | 8 |
| Fraksi metabolik (`meta_fr`) | Bagian bahan organik pupuk yang masuk pool metabolik pada `cswat = 2`. | 10 |
| Fraksi N/P biomassa (`pltnfr1..3`, `pltpfr1..3`, `n_fr`, `p_fr`) | Konsentrasi N dan P optimal pada awal, tengah dan akhir pertumbuhan. | 8 |

### G

| Istilah | Definisi | Bab |
| --- | --- | --- |
| Gage | File stasiun terukur untuk satu variabel iklim (hujan, suhu, radiasi, kelembapan, atau angin) yang ditempatkan ke larik harian; nama `sim` berarti variabel itu disimulasikan. | 4, 6 |
| Garis miring (`/`) | Pengakhir masukan list-directed; variabel sisanya tidak diubah. | 4 |
| Generator cuaca (weather generator, WGN, `weather-wgn.cli`) | Model stokastik yang membangkitkan cuaca harian dari parameter statistik iklim bulanan per stasiun bila data terukur tidak ada atau hilang; juga sumber elevasi rujukan lapse bila data disimulasikan. | 3, 4, 5, 6 |
| gfortran | Kompiler Fortran GNU yang dipakai membangun biner rujukan SWAT+; memanggil libm (glibc) untuk fungsi matematika. | 5 |
| glibc | Pustaka C GNU di Linux; libm-nya dipakai biner native dan acuan Fortran. | 16, 17 |
| Glue JavaScript | Kode perantara otomatis (mis. wasm-bindgen) yang tidak dipakai modul ini. | 16 |
| GNU long name ('L') | Entri tar khusus jenis `L` yang memuat nama berkas lebih dari 100 bita untuk entri berikutnya. | 15, 16 |
| Green dan Ampt (Green-Ampt, `gampt`) | Metode infiltrasi subharian berbasis fisika (konduktivitas dan hisapan front pembasahan) yang menggantikan bilangan kurva; dalam routing, limpasan per langkah HRU digeser sejauh waktu konsentrasi. | 11, 12, 17 |
| gsi | Konduktansi stomata maksimum tanaman. | 6 |

### H

| Istilah | Definisi | Bab |
| --- | --- | --- |
| Ha-m | Hektare-meter, satuan volume = 10.000 m3. | 13 |
| Hargreaves | Metode PET berbasis suhu dan radiasi ekstraterestrial. | 6 |
| Hari Julian (`jday`) | Nomor hari dalam tahun, 1 = 1 Januari, sampai 365 atau 366. | 3 |
| Hari menuju kematangan (`days_mat`) | Jumlah hari dari tanam ke matang di `plants.plt`; 0 berarti heat unit dihitung sepanjang tahun. | 8 |
| harv.ops | Basis data operasi panen: tipe, indeks panen (`hi_ovr`), efisiensi (`eff`), dan biomassa minimum (`bm_min`). | 8, 10 |
| Hasil aksi (`act_outcomes`) | Penanda `y`/`n` apakah sebuah aksi dijalankan untuk alternatif tertentu. | 13 |
| Heat unit (satuan panas, PHU, `phumat`, `phuacc`) | Akumulasi derajat-hari di atas suhu dasar tanaman. `phumat` adalah total kebutuhan menuju kematangan dan `phuacc` fraksi yang sudah terkumpul musim ini. | 3, 8, 10 |
| Heat unit dasar-0 (`phubase`, `phubase0`) | Akumulasi suhu di atas 0 °C sejak awal tahun sebagai fraksi dari total tahunan (`phutot`); dipakai untuk penjadwalan pengelolaan. | 6, 7, 10 |
| Heat unit seumur hidup (`phuacc_p`, `phumat_p`) | Fraksi heat unit perennial yang dihitung terhadap `phumat`·`mat_yrs`, dipakai untuk akar dan tinggi kanopi pohon. | 8 |
| `hi_ovr` | Indeks panen pengganti pada panen biomassa/residu. | 8 |
| Hidrograf (`Hyd`, `hyd_output`, `hd`) | Paket air (m3) beserta sedimen (t), nutrien (kg), dan suhu (°C) yang dikeluarkan sebuah objek per hari; struktur SWAT+ 18 komponen. HRU dan RU punya lima hidrograf (tot, rhg, sur, lat, til). | 3, 5, 12, 13, 14 |
| Hidrograf satuan (unit hydrograph, `uhyd`, `unit_hyd`) | Distribusi tak berdimensi limpasan sehari ke langkah subharian hari ini dan besok bila memakai bilangan kurva; berbentuk segitiga (`uhyd` 0) atau gamma (`uhyd` 1). | 11, 12, 17 |
| hin / hin\_d | Jumlah inflow harian sebuah objek dan daftar masing-masing inflow-nya. | 12 |
| Hisapan front pembasahan (`wfsh`) | Tekanan kapiler di batas tanah basah-kering dalam model Green dan Ampt (mm). | 11 |
| HRU (hydrologic response unit) | Unit respons hidrologi: satuan lahan dengan kombinasi unik tanah, tutupan/tata guna lahan, dan lereng. Tempat neraca air, tanaman, hara, dan erosi dihitung harian (`Hru::day`). | 3, 4, 5, 7 |
| `hru_wb`, `hru_pw`, `hru_nb`, `hru_ls` | Tabel keluaran harian neraca air, tanaman-cuaca, neraca nutrien, dan kehilangan. | 7 |
| HruRows | Kumpulan nilai baris tabel HRU (wb, nb, ls, pw) yang sudah dihitung engine. | 15 |
| Hujan per langkah (`ts`, `ts_next`, `sta_ts`) | Curah hujan setiap langkah waktu subharian (mm), hari ini (`ts`) dan esok (`ts_next`), setelah intersepsi kanopi atau dari stasiun (`sta_ts`). | 6, 11 |
| Humus aktif (`hact`) | Kolam humus yang mudah dimineralisasi pada model karbon asli (`cswat` 0), dengan fraksi `fr_hum_act`; juga menerima biji yang luput dari panen biji. | 8, 9, 10 |
| Humus lambat (HS, `hs`) | Kolam humus dengan waktu tinggal puluhan tahun. | 9 |
| Humus pasif (HP, `hp`) | Kolam humus dengan waktu tinggal ratusan tahun. | 9 |
| Humus stabil (`hsta`) | Kolam humus yang lambat berubah pada model karbon asli (`cswat` 0). | 9, 10 |
| hvkl | Operasi panen sekaligus mematikan tanaman. | 10 |
| hyd\_flo | Hidrograf debit per langkah sebuah objek untuk satu atau dua hari. | 12 |
| hyd\_hdr | Header 18 kolom hidrograf SWAT+: flo, sed, orgn, sedp, no3, solp, chla, nh3, no2, cbod, dox, san, sil, cla, sag, lag, grv, dan suhu berlabel `null`. | 15 |
| hydout\_add | Operator `+` hidrograf; jumlah demi jumlah, suhu rata-rata berbobot aliran. | 5 |
| hydrop | Skema operasi PLTA ala HYPE dengan faktor penyimpanan F\_lin dan faktor musiman F\_sin. | 13, 17 |

### I

| Istilah | Definisi | Bab |
| --- | --- | --- |
| I\_mean | Rata-rata inflow jangka panjang (lima tahun bergulir) untuk skema Hanasaki/HYPE; di SWAT+ 6f51536 selalu nol. | 13 |
| idc\_till | Kode metode efek pengolahan terhadap dekomposisi (1 DSSAT, 3 berbasis `tillagef`). | 9 |
| idg | Pemetaan tarikan ke slot benih; diacak bila `igen` tidak nol. | 6 |
| igen | Kode pembangkit bilangan acak di `parameters.bsn` (bawaan 5); nilai selain 0 membuat `gcycl` menggeser dan mengacak benih generator cuaca sehingga deret cuaca berbeda. | 3, 4, 6, 17 |
| igropt | Pilihan cara menggabungkan faktor cahaya dan nutrien pada pertumbuhan alga (perkalian, minimum, harmonik). | 12 |
| imax | Jumlah baris atau nomor terbesar yang dihitung SWAT+ pada lintasan pertama untuk mengalokasikan larik sebelum lintasan baca. | 4 |
| Imobilisasi (`imm`) | Pengikatan N/P mineral ke dalam biomassa atau bahan organik ketika aliran karbon miskin nutrien. | 9 |
| Implicit SAVE | Variabel lokal Fortran yang mempertahankan nilai antarpanggilan, sumber keadaan yang terbawa antar-HRU. | 17 |
| Indeks luas daun (LAI, `lai`) | Luas daun per luas lahan (m2/m2) komunitas tanaman. | 7, 8 |
| Indeks panen (HI, `hvsti`, `harv_idx`, `hi_adj`) | Fraksi biomassa di atas tanah yang menjadi hasil atau biji; `hi_adj` adalah nilai yang berkembang selama musim. | 8, 10 |
| Infiltrasi kumulatif (`cuminf`) | Jumlah air yang telah meresap sejak awal hari pada model Green dan Ampt (mm). | 11 |
| inflo\_rate / inflo\_frac | Opsi pelepasan berbasis inflow hari itu (inflow plus tambahan dengan batas minimum, atau fraksi inflow). | 13 |
| inflo\_targ | Opsi pelepasan inflow ditambah kelebihan di atas fraksi volume utama (target). | 13, 17 |
| inspect | Perintah CLI yang membaca skenario dan mencetak ringkasan tanpa simulasi. | 16 |
| Intersepsi kanopi (`canstor`, `canmx`) | Air hujan yang tertahan daun; `canmx` adalah simpanan intersepsi maksimum pada LAI maksimum (mm). | 7, 14 |
| iostat | Kode status `read` Fortran: negatif berarti akhir file (EOF), positif galat konversi. SWAT+ umumnya hanya memeriksa akhir file (`iostat < 0`). | 3, 4 |
| ires1 / ires2 | Bulan awal dan akhir musim pengendapan pertengahan tahun. | 13 |
| Irigasi (`irrm`, `irrigate`, `irr.ops`) | Pemberian air dengan efisiensi `eff` dan fraksi limpasan `surq`. | 10 |
| Irigasi genangan (ponding) | Irigasi sawah yang mengisi kembali genangan sampai kedalaman sasaran. | 10 |
| irr\_applied, irr\_runoff | Air irigasi yang masuk tanah dan yang menjadi limpasan pada hari itu (mm). | 10 |
| irr\_yr | Irigasi kumulatif HRU; tidak pernah di-reset di SWAT+ (di-reset tahunan dengan `--fix-swat`). | 10 |
| iyres / mores | Tahun dan bulan mulai beroperasinya reservoir. | 13 |

### J

| Istilah | Definisi | Bab |
| --- | --- | --- |
| Jadwal pengelolaan (`management.sch`, `Mgt`) | Daftar operasi melingkar per HRU (tanam, panen, olah tanah, pupuk, irigasi) bertanggal atau berbasis heat unit, beserta tabel keputusan otomatisnya; penunjuk `cur_op` menandai operasi berikutnya. | 3, 4, 10 |
| Jalur filter vegetatif (VFS, `filterstrip.str`) | Jalur bervegetasi di tepi lahan yang menahan sedimen dan hara dari limpasan. | 11 |
| Jari-jari hidraulik (R) | Luas penampang dibagi keliling basah. | 12 |
| jdt | Fungsi hari Julian dari bulan dan tanggal. | 10 |
| Jendela cetak (`print.prt`: `day_start`, `day_end`, `yrc_start`, `yrc_end`, `interval`) | Rentang dan interval hari yang dicetak pada tabel harian. | 5 |
| Jenis hidrograf (`hyd_typ`, `ihtyp`: tot, rhg, sur, lat, til) | Komponen hidrograf yang dialirkan pada sebuah sambungan: total, recharge (perkolasi ke akuifer), limpasan permukaan, aliran lateral, dan drainase tile. Untuk akuifer, `tot` = aliran balik dan `rhg` = rembesan. | 3, 4, 12, 14 |
| JSON lines | Format satu objek JSON per baris, dipakai untuk progres dan kejadian. | 15 |

### K

| Istilah | Definisi | Bab |
| --- | --- | --- |
| k (konduktivitas dasar reservoir) | Konduktivitas hidrolik dasar reservoir (mm/jam) untuk menghitung rembesan. | 13 |
| K11 | Keputusan desain RustGIS bahwa engine LGPL selalu dijalankan sebagai program atau modul terpisah, tidak ditautkan. | 16 |
| Kalibrasi keras (hard calibration) | Perubahan parameter langsung dari `calibration.cal` (absval, abschg, pctchg, relchg) dengan batas di `cal_parms.cal`, diterapkan sekali setelah objek disiapkan tanpa iterasi otomatis. | 3, 4, 7, 14 |
| Kalibrasi lunak (soft calibration, `calsoft_*`) | Penyesuaian iteratif SWAT+ terhadap rasio neraca air/sedimen target; tidak diport. | 14 |
| Kapasitas air tersedia (AWC, `awc`) | Selisih kapasitas lapang dan titik layu lapisan tanah, dalam mm air per mm tanah. | 3, 4, 7, 14 |
| Kapasitas angkut (`trancap`) | Batas sedimen runon yang dapat diteruskan, bergantung `dep_co`, faktor C USLE, debit, kemiringan dan lebar lahan. | 12 |
| Kapasitas lapang (`fc`, `sumfc`) | Air tanah di atas titik layu yang tertahan setelah drainase bebas (mm). | 7 |
| Kebiasaan SWAT+ (quirk) | Perilaku SWAT+ yang mengejutkan atau keliru tetapi sengaja ditiru port agar hasil sama. | 17 |
| Kedalaman akar (`root_dep`, `rdmx`) | Kedalaman akar saat ini (mm) dan kedalaman akar maksimum tanaman dari `plants.plt` (m). | 8, 10 |
| Kedalaman dasar akuifer (`dep_bot`, m) | Jarak dari permukaan ke dasar akuifer. | 14 |
| Kedalaman muka air tanah (`dep_wt`, m) | Jarak dari permukaan ke muka air, dihitung dari simpanan dan specific yield. | 14 |
| Kedalaman redaman (DD) | Kedalaman tempat fluktuasi suhu permukaan teredam, untuk suhu tanah. | 7 |
| Kegagalan septik (`opt` = 2, `tsincefail`) | Keadaan biozona tersumbat (kejenuhan ≤ kapasitas lapang) sehingga hampir tidak meloloskan air. | 11 |
| Kehilangan transmisi (`trans_loss`, `chk`) | Rembesan air dari dasar saluran, chk·L·W·24 m3/hari. | 12 |
| Kejenuhan (`ul`, `sumul`) | Air di atas titik layu bila semua pori terisi (mm). | 7 |
| Kelas butir sedimen (`san`, `sil`, `cla`, `sag`, `lag`, `grv`) | Pasir, debu, liat, agregat kecil, agregat besar, dan kerikil (t) yang menyusun hasil sedimen. | 11, 13 |
| Kelebihan jenuh (saturation excess, `satex`, `ul_excess`) | Air yang melampaui kejenuhan lapisan atas; naik menjadi limpasan atau dikembalikan ke genangan oleh `swr_satexcess`. | 7, 11 |
| Kelompok hidrologi tanah (HSG, `hydgrp`) | Klasifikasi tanah A–D menurut potensi limpasan dalam metode bilangan kurva. | 3, 4, 14 |
| Keluaran list-directed (`write (unit,*)`) | Keluaran Fortran tanpa format eksplisit; runtime memilih lebar dan bentuk angka. | 15 |
| Kemiringan kurva tekanan uap jenuh (Δ) | Kemiringan kurva tekanan uap jenuh terhadap suhu (kPa/°C). | 6 |
| Kemiringan sisi (`chss`, z) | Rasio horizontal terhadap vertikal tebing alur trapesium. | 12 |
| Kepekaan lepas tanah (`erod_k`) | Koefisien detachability tanah (g/J) yang dipilih dari tekstur. | 11 |
| Kerapatan trotoar (`curbden`) | Panjang trotoar per luas (km/ha), mengubah padatan per km trotoar menjadi per ha. | 11 |
| kill (`mgt_killop`) | Mematikan tanaman; bagian atas menjadi residu permukaan, akar menjadi residu lapisan. | 10 |
| Klorofil-a / biomassa alga (`chla`, `ai0`) | Klorofil-a dan rasio klorofil terhadap biomassa alga. | 12 |
| KOC (`prmt_21`) | Koefisien partisi karbon organik yang mengatur pelepasan C mikroba ke air. | 9 |
| Kode keluar (exit code) | 0 berhasil, 1 galat atau ketidakcocokan, 2 argumen salah, 3 tidak tersimulasi (strict). | 16 |
| Koefisien ekspor (exco) | Masukan muatan rata-rata tahunan konstan (recall tipe 4). | 4 |
| Koefisien evaporasi badan air (`evrsv`) | Pengali PET untuk evaporasi permukaan air terbuka reservoir dan lahan basah. | 11, 13 |
| Koefisien kurva retensi (`wrt`) | Dua koefisien kurva-S yang menghubungkan air tanah profil dengan parameter retensi harian. | 7 |
| Koefisien pemadaman cahaya (`ext_coef`) | Parameter hukum Beer untuk intersepsi cahaya oleh kanopi (bawaan 0,65). | 8 |
| Koefisien pertukaran panas (`k_e`) | Fungsi suhu, angin, dan titik embun; satuan W/(m2·°C). | 12 |
| Koliform tinja (`fcoli`) | Bakteri indikator pencemaran tinja (cfu/100 mL). | 11 |
| Komunitas tanaman (`plant.ini`) | Kumpulan tanaman per HRU beserta kondisi awalnya (LAI, biomassa, residu awal). | 3, 4 |
| Kondisi (`cond`, `CondVar`) | Variabel keadaan yang dibandingkan dengan batas `lim_const` memakai operator `<`, `>`, `<=`, `>=`, `=`, `/=`. | 10 |
| Kondisi kalibrasi (`CalCond`) | Syarat teks pada HRU (kelompok hidrologi, tekstur, tata guna lahan, grup kalibrasi, tanaman) agar perubahan diterapkan. | 14 |
| Konduktivitas efektif (`adj_hc`, K\_e) | Konduktivitas jenuh lapisan atas yang disesuaikan bilangan kurva untuk Green dan Ampt (mm/jam). | 11 |
| Konduktivitas hidraulik jenuh (`k`, Ksat) | Laju aliran air melalui lapisan tanah jenuh (mm/jam). | 3, 14 |
| Konstanta psikrometrik (γ) | Parameter yang menghubungkan tekanan uap dan suhu udara (kPa/°C). | 6 |
| Konstanta resesi aliran dasar (`alpha`) | Konstanta (1/hari) yang menentukan laju penurunan aliran balik dari akuifer dangkal; dipakai sebagai `alpha_e = exp(-alpha)`. | 3, 14 |
| Kurva rating (rating curve, kurva debit, `rcurv`, `Rc`) | Tabel empat titik kedalaman kanal SWAT-DEG (0,1 bankfull, bankfull, 1,2 dan 2 kali bankfull) berisi debit, luas, volume, kecepatan, dan waktu tempuh. | 12, 17 |
| Kurva volume–luas (`br1`, `br2`) | Hubungan pangkat A = br1·V^br2 antara volume tampungan (m3) dan luas permukaan (ha). | 13 |
| Kurva-S (`ascrv`, `leaf1`, `leaf2`) | Fungsi f(x) = x/(x + exp(a − b x)) yang koefisiennya ditentukan dari dua titik; dipakai untuk LAI, RUE–CO₂ dan fraksi N/P. | 8 |

### L

| Istilah | Definisi | Bab |
| --- | --- | --- |
| Lag pelepasan (`lag_up`, `lag_down`) | Parameter peredam perubahan pelepasan harian, dipakai sebagai α = exp(−lag); dibaca dari kolom `br1`/`br2` `hydrology.res`. | 13 |
| Lahan basah HRU (surface storage, `surf_stor`) | Tampungan air tergenang di atas permukaan tanah sebuah HRU, disimulasikan seperti waduk kecil dengan rembesan, evaporasi, dan pelepasan. | 11 |
| LAI potensial (`blai`, `lai_pot`) | LAI maksimum tanaman pada kondisi tanpa stres. | 8 |
| `laixco_tree` | Koefisien yang membatasi LAI maksimum pohon muda menurut umur. | 8 |
| Laju lapse (lapse rate, `plaps`, `tlaps`) | Laju perubahan presipitasi (mm/km) dan suhu (°C/km) terhadap selisih elevasi, untuk mengoreksi data stasiun atau pengukur ke elevasi objek. | 3, 5, 6 |
| Langkah subharian (`time%step`, `step`, `dtm`) | Pembagian hari menjadi beberapa langkah waktu (misalnya 24 per jam); `step` di atas 1 mengaktifkan simulasi subharian dan `dtm` = 1440/step menit. Aksi pelepasan reservoir diulang sekali per langkah. | 3, 6, 11, 12, 13 |
| `lat_ttime` | Waktu tempuh aliran lateral, disimpan sebagai bagian simpanan yang dilepas per hari, 1 − exp(−1/t). | 7, 14 |
| Latin-1 | Pengodean byte tunggal; dipakai port sebagai cadangan bila file bukan UTF-8 sah. | 4 |
| LGPL-2.1 | Lisensi engine; alasan engine dijalankan sebagai program/modul terpisah dan tanpa dependensi luar. | 15 |
| libm | Pustaka fungsi matematika (exp, ln, pow, trigonometri); berbeda antara glibc, musl/Rust libm, macOS, dan Windows. | 16 |
| Liebig (hukum minimum) | Prinsip bahwa pertumbuhan dibatasi faktor yang paling kurang; dasar `reg` = min stres. | 8 |
| little-endian (LE) | Urutan bita dengan bita paling rendah lebih dulu; dipakai untuk 4 bita panjang keluaran. | 16 |
| lnvol | Selisih log10 volume darurat dan utama, disimpan sebagai bilangan bulat di SWAT+. | 13 |
| LoadReport | Laporan pemuatan berisi file yang hilang (`missing`) dan file iklim yang tidak terbaca (`unread`). | 4, 16 |
| Loop perintah (`command`) | Loop harian SWAT+ atas semua objek ruang dalam urutan perintah (`cmd_next`), diikuti panggilan semua rutin keluaran. | 5 |
| Loop simulasi (`time_control`) | Loop tahun dan hari SWAT+ yang memanggil iklim, objek lanskap dan routing, serta rutin keluaran; di port diwujudkan oleh `run_with`. | 5 |

### M

| Istilah | Definisi | Bab |
| --- | --- | --- |
| Masukan list-directed (`read (unit, *)`) | Cara baca Fortran tanpa format tetap yang mengisi variabel berurutan dari nilai yang dipisah spasi, tab, atau koma; aturannya (lanjut baris, null, garis miring, pengulangan) menentukan arti file SWAT+. | 3, 4 |
| `mat_yrs`, `curyr_mat` | Jumlah tahun perennial menuju dewasa dan umur saat ini (tahun). | 8 |
| Matriks A dan B | Koefisien model autoregresif multivariat orde satu yang memberi korelasi seri dan silang antarvariabel cuaca. | 6 |
| meas | Opsi pelepasan terukur dari objek recall, tidak didukung port. | 13 |
| Memori linear (`memory`) | Satu larik bita yang dapat tumbuh, milik modul WebAssembly, tempat penyangga masukan dan keluaran berada. | 16 |
| MemSource / DirSource | Implementasi `FileSource`: sumber berkas proyek dari memori (WASM) atau dari folder di disk (CLI) bagi `load_project`. | 4, 16 |
| MemStore | Store yang menyimpan berkas tabel keluaran dalam peta nama ke bita di memori, untuk WebAssembly. | 15, 16 |
| Metode Schrage | Teknik menghitung (a·s) mod m tanpa luapan bilangan bulat 32 bit. | 5 |
| Mineralisasi (`hmn`, `act_nit_n`) | Perubahan N/P organik menjadi bentuk mineral (nitrat, P labil) oleh dekomposisi mikroba. | 9 |
| minp (P mineral) | Di status harian berisi beban P terlarut dari resapan dikali konsentrasi basis data (kg). | 14 |
| Mononobe | Distribusi hujan jam-jaman dengan kedalaman kumulatif R (t/T)^(1/3). | 17 |
| msk\_co1, msk\_co2, msk\_x | Bobot K pada bankfull dan 0,1 bankfull serta faktor X Muskingum di `parameters.bsn`. | 12 |
| MSRV (minimum supported Rust version) | Versi Rust minimum yang didukung, 1.90. | 16, 17 |
| Muka air dangkal (`wt_shall`) | Kedalaman zona jenuh yang diperkirakan dari air tanah di atas kapasitas lapang. | 7 |
| Muskingum (`ch_rtmusk`, `rte` 1) | Metode routing kanal dengan tampungan O2 = C1 I2 + C2 I1 + C3 O1, memakai konstanta waktu tampung K dan faktor bobot X. | 12, 17 |
| musl | Pustaka C alternatif; fungsi matematika Rust di WebAssembly adalah port darinya, sehingga pembulatan bit terakhir dapat berbeda dari glibc. | 16, 17 |
| MUSLE | Modified Universal Soil Loss Equation (Williams 1975): hasil sedimen harian dari volume limpasan dan debit puncak. | 7, 9 |

### N

| Istilah | Definisi | Bab |
| --- | --- | --- |
| N dan P tanaman (`nplnt`, `nplt`, `pplnt`) | N dan P yang diserap tanaman (kg/ha), total berjalan per HRU. Pada periode HRU dicetak nilai hari terakhir; `pplnt` cekungan selalu 0. | 5, 8 |
| N/P optimal (`un2`, `up2`) | Kandungan N tanaman dan P di atas tanah yang optimal (kg/ha). | 8 |
| n\_act\_frac (fr\_actN) | Fraksi N humus yang berada di kolam aktif dalam keseimbangan; 0,02 bawaan. | 9 |
| nam1 | Kode `codes.bsn` yang menambah kolom konstituen di `soil_plant.ini`. | 4 |
| NaN | Nilai float bukan-angka; dipakai Parquet port untuk NaN atau tanda bintang luapan Fortran. | 15 |
| natlake | Opsi pelepasan danau alami (Döll dkk. 2003), Q = kr·(S/(Smax − So))^1,5·S. | 13, 17 |
| nbyr | Jumlah tahun kalender yang disimulasikan, `yrc_end − yrc_start + 1`. Dalam kepala file deret iklim terukur, `nbyr` dan `tstep` menyatakan jumlah tahun dan langkah waktu per hari. | 3, 4, 5 |
| ndays (`ndays_leap`, `ndays_noleap`) | Tabel global hari kumulatif pada akhir setiap bulan (13 elemen) untuk tahun kabisat atau biasa; diubah oleh pembaca suhu. | 3, 5, 6 |
| Nilai hilang (-99, -98) | -99 ditulis di file untuk data hilang; -98 menandai hari di luar rentang file; keduanya memicu generator (ambang <= -97). | 6 |
| Nilai null (`,,`) | Nilai kosong di masukan list-directed yang membiarkan variabel bernilai sebelumnya (default). | 4 |
| Nitrat lateral (`latno3`) | Nitrat yang terbawa aliran lateral. | 9 |
| Nitrifikasi (`rnit`) | Oksidasi amonium menjadi nitrat, bergantung suhu dan kadar air (`nut_nitvol`); di biozona septik dihitung dengan laju orde satu sebanding biomassa. | 9, 11 |
| no3\_lat / no3\_seep / no3\_loss / no3\_rchg (kg/ha) | Nitrat yang keluar bersama aliran balik, bersama rembesan, hilang karena peluruhan, dan masuk bersama resapan. | 14 |
| no3\_st (kg/ha) | Nitrat tersimpan di akuifer. | 14 |
| nonirr-h06 / irr-h06 | Skema operasi waduk Hanasaki dkk. (2006) untuk waduk non-irigasi dan irigasi. | 13, 17 |
| `nostress` | Kode `codes.bsn` yang mematikan semua stres (1) atau hanya stres N dan P (2). | 8 |
| nperco (β\_NO3) | Koefisien nitrat yang ikut limpasan permukaan. | 9 |
| nsed | Konsentrasi sedimen normal (keseimbangan) reservoir, mg/L. | 13 |
| nsetlr / psetlr | Laju kehilangan (pengendapan) N organik dan P sedimen, per tahun di file dan per hari di model. | 13 |
| nsolr / psolr | Laju kehilangan N dan P terlarut. | 13 |
| num\_actions | Penghitung pelaksanaan aksi per tahun, mulai 1 dan dibatasi `Int(const2)`. | 10 |
| num\_leap | Jumlah tahun kabisat yang disimulasikan; dipakai dalam pembagian bulat `num_leap / nbyr`. | 5 |
| nyskip | Jumlah tahun awal simulasi (pemanasan, warm-up) yang disimulasikan tetapi tidak dicetak. | 3, 4, 5, 13, 15, 16, 17 |

### O

| Istilah | Definisi | Bab |
| --- | --- | --- |
| ob\_typ | Jenis objek pemilik parameter kalibrasi (`hru`, `sol`, `bsn`, `aqu`, `plt`, `cli`, `rdt`, dst.). | 14 |
| obj\_typ | Kode jenis objek penerima dalam daftar aliran keluar file koneksi. | 3 |
| object.cnt | File hitungan objek per jenis dan luas cekungan; menentukan file koneksi mana yang dibaca dan berapa baris. | 4 |
| Objek ruang (objek spasial, `ob`) | Setiap unit yang dirouting SWAT+ (HRU, unit routing, akuifer, kanal, reservoir, recall, outlet, dsb.), satu baris file `*.con`; disimpan dalam satu larik dengan urutan jenis tetap. | 3, 12 |
| Oksigen jenuh (`soxy`) | Konsentrasi DO jenuh sebagai fungsi suhu (persamaan APHA). | 12 |
| om\_water.ini / initial.res | File kondisi awal kualitas air dan penunjuk kondisi awal reservoir. | 13 |
| Operasi berbasis heat unit (`husc`, `hu_sch`) | Operasi yang dijalankan begitu heat unit tanaman pertama (atau heat unit dasar-0 tahunan) melewati ambangnya. | 10 |
| Operasi bertanggal | Operasi jadwal dengan bulan dan tanggal yang dijalankan `mgt_operatn` pada hari itu. | 10 |
| Operator `+`, `/`, `//` | Operator SWAT+ pada tipe keluaran: jumlah harian, bagi tahun tercetak (fluks), bagi jumlah hari (keadaan). | 5 |
| OrgMass (`organic_mass`) | Rekaman massa organik {massa, C, N, P} dalam kg/ha, dipakai untuk residu, bahan organik, dan kolam karbon. | 8, 9, 10 |
| orgn / sedp / no3 / nh3 / no2 / solp | N organik, P terikat sedimen, nitrat, amonium, nitrit, P terlarut (kg). | 13 |
| out\_tot | Jumlah aliran keluar sebuah objek di file koneksi. | 4 |
| Outlet (`out`) | Objek ujung yang meneruskan inflow tanpa perubahan. | 12 |
| OutputSink | Trait Rust di `swat-engine/src/run.rs` yang menjadi satu-satunya jalur keluaran engine; menerima baris harian dan periode pada hari tercetak dan akhir periode, diimplementasikan oleh `TextTables`. | 5, 15 |

### P

| Istilah | Definisi | Bab |
| --- | --- | --- |
| P labil (`lab`) | P larut/tersedia dalam larutan tanah, kg/ha. | 9 |
| P mineral aktif (`act`) | P mineral yang cepat berkeseimbangan dengan P labil. | 9 |
| P mineral stabil (`sta`) | P mineral yang lambat berkeseimbangan dengan P aktif. | 9 |
| Padatan maksimum (`dirtmx`) | Batas akumulasi padatan di permukaan kedap air (kg/km trotoar). | 11 |
| Panen biji (`grain`, `picker`, `stripper`) | Panen biji; sisanya masuk humus aktif lapisan atas. | 10 |
| Panen biomassa (`biomass`) | Panen fraksi `hi_ovr·eff` dari biji, daun dan batang. | 10 |
| Panen residu (`residue`) | Pengangkutan sebagian residu permukaan semua tanaman. | 10 |
| Panen umbi (`tuber`, `peanuts`) | Panen fraksi akar; sisa tanaman menjadi residu. | 10 |
| PAR1 | 4 bita magic di awal dan akhir berkas Parquet. | 15 |
| Parameter retensi (`smx`, S) | Retensi maksimum (mm) pada kondisi kelembapan I; S harian diturunkan dari air tanah lewat kurva retensi. | 7 |
| parameters.bsn | File parameter real tingkat basin (42 parameter) ditambah `day_lag_mx` dan `igen`. | 3, 4 |
| Park–Miller (minimal standard, `aunif`) | Generator bilangan acak kongruensial perkalian s(n+1) = 16807·s(n) mod (2^31 − 1), diimplementasikan `aunif` dengan metode Schrage. | 5, 6 |
| Parquet (Apache Parquet) | Format berkas tabel kolumnar biner. Port menulis subset datar, REQUIRED, PLAIN, tanpa kompresi, dengan kunci int32, nama UTF-8, dan nilai float32. | 15, 16, 17 |
| Partisi (`pl_partition`) | Pembagian pertambahan biomassa ke akar, biji, daun dan batang. | 8 |
| pcf | Faktor koreksi hujan bulanan agar rerata hujan yang dibangkitkan sama dengan rerata masukan. | 6 |
| pco | Struktur Fortran berisi kode cetak hasil pembacaan `print.prt`. | 15 |
| pcp\_days | Rerata jumlah hari basah per bulan. | 6 |
| pcpmean | Rerata hujan per hari basah (mm). | 6 |
| Pelimpah darurat (*emergency spillway*, `evol`, `esa`) | Ketinggian muka air banjir maksimum; volume dan luas pada ketinggian itu. | 13 |
| Pelimpah utama (*principal spillway*, `pvol`, `psa`) | Ketinggian muka air normal; `pvol` volume (ha-m di file, m3 di model) dan `psa` luas (ha) pada ketinggian itu. | 13 |
| Peluang hari basah (`pr_wd`, `pr_ww`, `wet_dry`, `wet_wet`) | Peluang hari basah setelah hari kering, P(W\|D), dan setelah hari basah, P(W\|W), dalam bulan tertentu; parameter rantai Markov orde satu generator cuaca. | 3, 6 |
| Pelumpuran (puddling, `puddle.ops`) | Pengolahan tanah sawah di bawah genangan (`mgt_newtillmix_wet`) yang juga mengubah konduktivitas lapisan atas dan sedimen air. | 9, 10, 17 |
| Pembelahan lapisan (`layersplit`) | Penyisipan batas lapisan baru pada kedalaman tertentu, dipakai untuk membentuk lapisan biozona. | 11 |
| Pencampuran biologis (`biomix`, `bmix`) | Pengadukan tanah lapisan atas oleh biota (cacing dan sebagainya) yang memindahkan air dan pool; `biomix` adalah efisiensinya. | 7, 9 |
| Pencarian biner (`search`) | Pencarian nama dalam daftar terurut dengan paling banyak 25 pembagian dua; dapat gagal pada daftar tak terurut. | 5 |
| Penggelontoran (flush) | Kondisi ketika genangan yang semula kosong melimpas; sedimennya dihitung dengan bentuk MUSLE. | 11 |
| Pengolahan tanah (tillage, `tillage.til`) | Pencampuran tanah oleh alat dengan efisiensi pencampuran `effmix` dan kedalaman `deptil` (mm). | 9, 10 |
| Penman-Monteith | Metode PET kombinasi energi–aerodinamik dengan tahanan kanopi dan aerodinamik. | 6 |
| Penormal serapan (`water_norm`, `n_norm`, `p_norm`) | 1 − e^(−β) untuk menormalkan distribusi serapan. | 8 |
| Penumpukan dan pencucian (build-up/wash-off) | Model akumulasi padatan di permukaan kedap air antara hujan dan pencuciannya oleh limpasan. | 7, 11 |
| Perbandingan string berpengisi spasi | Aturan Fortran yang menganggap string pendek diisi spasi sampai panjang deklarasi (50 karakter untuk nama). | 5 |
| `perco` / `perco_lim` | Koefisien perkolasi HRU (`hydrology.hyd`) dan batas drainase dasar profil yang diturunkan darinya lewat fungsi eksponensial ganda. | 7, 14, 17 |
| Period | Periode keluaran `Mon`, `Yr`, `Aa`; dengan `None` untuk harian; akhiran berkas `day`, `mon`, `yr`, `aa`. | 15 |
| PeriodDay | Fakta kalender harian (`end_mo`, `end_yr`, `end_sim`, hari bulan, `day_end_yr`, `yrs_prt`, `days_prt`) untuk akumulator. | 5 |
| Perkolasi (`prk`, `sepbtm`) | Air yang turun antarlapisan; `sepbtm` keluar dari dasar profil menuju akuifer (mm). | 7 |
| Permintaan basi | Bug `pl_nut_demand` yang menolkan elemen array yang salah sehingga tanaman memakai permintaan HRU lain. | 8 |
| Permintaan N/P (`uno3d`, `uapd`) | Kebutuhan serapan N dan P hari ini (kg/ha). | 8 |
| Persamaan Manning (`qman`) | Rumus debit aliran terbuka Q = A R^(2/3) S^(1/2) / n dari luas penampang, jari-jari hidraulik, kekasaran n, dan kemiringan; SWAT+ memakai eksponen 0,6666 dan n + 0,001. | 11, 12 |
| `pet_co` | Koefisien pengali PET tingkat HRU. | 6, 7 |
| phoskd (k\_d) | Koefisien partisi P tanah yang mengatur P larut dalam limpasan. | 9 |
| phutot | Jumlah satuan panas tahunan basis 0 °C dari suhu rerata bulanan (°C·hari). | 6 |
| pl\_yield | Hasil panen terakhir HRU (kg/ha), dijumlahkan ke `yield_tot` tanaman. | 10 |
| PLAIN encoding | Encoding Parquet paling sederhana: angka little-endian, string = panjang 4 bita + bita. | 15 |
| Plak (`plqm`) | Bahan organik dan bakteri mati yang menyumbat pori biozona (kg/ha). | 11 |
| plant\_init | Inisialisasi tanaman yang juga menentukan operasi jadwal pertama. | 10 |
| Porositas (`por`) | Fraksi volume pori, 1 − bd/2,65. | 7 |
| powf | Fungsi pangkat real pustaka C; `x ** y` gfortran (mis. `x ** 0.5`) memanggilnya, dan port memanggilnya lewat `fortran::rpow`. | 5, 17 |
| pperco | Koefisien perkolasi P. | 9 |
| ppet / p\_pet\_rto | Akumulasi rasio hujan/PET harian stasiun dan rasio jumlah hujan terhadap PET dalam jendela 30 hari generator (indeks kelembapan iklim). | 6 |
| ppm | Bagian per juta, setara mg/L (g/m3) untuk air. | 13 |
| pr\_wdays | Fraksi hari basah dalam bulan (`pcp_days / hari dalam bulan`). | 6 |
| `precip_half_hr` (α0,5) | Fraksi hujan harian yang jatuh pada setengah jam terderas; dipakai untuk laju puncak limpasan dan bentuk badai. | 6, 7, 9 |
| precip\_mce / pet\_mce | Jendela bergerak 30 hari hujan dan PET untuk indeks kelembapan. | 6 |
| precip\_next | Hujan hari berikutnya yang sudah dibaca/dibangkitkan; SWAT+ memutuskan hujan satu hari di muka. | 6 |
| Presisi tunggal (`real`, `f32`) | Bilangan titik-mengambang 32 bit IEEE 754, setara `real` bawaan SWAT+; dipakai port agar hasil sama bit demi bit, dengan teks dibulatkan sekali ke f32. | 3, 4, 5 |
| Priestley-Taylor | Metode PET berbasis radiasi netto dengan koefisien α = 1.28. | 6 |
| print-daily.sh | Skrip validasi yang mengubah `print.prt` agar tabel objek tertentu dicetak harian sampai rata-rata tahunan. | 16, 17 |
| print.prt | File pengaturan keluaran SWAT+: tahun pemanasan (`nyskip`), jendela cetak, interval, dan bendera tabel harian, bulanan, tahunan, dan rata-rata tahunan per objek. | 3, 4, 15, 16 |
| PrintGate (`day_print`, `day_print_over`, `int_day_cur`) | Status saklar cetak harian SWAT+ yang ditiru port. | 5 |
| prob\_unif | Kondisi tabel keputusan berpeluang yang mengundi satu hari acak berdistribusi seragam dalam jendela hari, dengan peluang `frac_app` yang dibaca dari kolom objek. | 4, 10, 17 |
| --progress json | Opsi `run` yang menulis event JSON per baris (start, progress, note, table, done, error) ke stdout. | 16 |
| Ps\_coast | Potongan dataran rendah pantai Pessel (74 kanal, 115 HRU, 2015 sampai 2017), dasar hampir semua varian Ps\_\*. | 17 |
| PSP (π, `psp`) | Indeks ketersediaan P (phosphorus sorption ratio) pada model P; fraksi pupuk P yang tetap labil setelah keseimbangan. | 9, 17 |
| Pupuk kandang (`manu`, `manure_db.frt`, `manure_om.frt`) | Pupuk organik dengan kandungan air `frac_water`. | 10 |
| PW\_NAMES | 25 nama kolom tabel tanaman dan cuaca (`_pw`). | 15 |

### Q

| Istilah | Definisi | Bab |
| --- | --- | --- |
| QUAL2E | Model kualitas air sungai EPA (alga, siklus N dan P, CBOD, oksigen) yang diadopsi SWAT dan dipakai `ch_watqual4`; parameternya disimpan di `nutrients.cha`. | 3, 12, 17 |

### R

| Istilah | Definisi | Bab |
| --- | --- | --- |
| Radiasi aktif fotosintesis terintersepsi (PAR, `par`) | Bagian radiasi matahari (0,5 × radiasi total) yang ditangkap kanopi menurut hukum Beer. | 8 |
| Radiasi netto gelombang pendek / panjang (R\_ns, R\_nl) | Radiasi matahari terserap setelah albedo, dan pertukaran radiasi termal bersih permukaan (MJ/m²/hari). | 6 |
| rain\_yrs | Jumlah tahun data untuk hujan setengah jam maksimum. | 6 |
| rainhmx (`pcp_hhr`) | Curah hujan maksimum setengah jam rata-rata bulanan (mm) dalam statistik generator cuaca. | 3, 4 |
| Rantai Markov basah/kering | Model orde satu yang menentukan hari hujan dari peluang P(W\|D) dan P(W\|W) bergantung status hari sebelumnya. | 6 |
| Rasio lahan terhadap jalur (`fld_vfs`, `ratio`) | Perbandingan luas lahan yang mengalir ke jalur dengan luas jalur. | 11 |
| Rasio pengayaan (enrichment ratio, `enratio`) | Rasio konsentrasi zat atau nutrien pada sedimen tererosi terhadap tanah asal, dibatasi 3; di SWAT+ variabelnya dipakai bersama antar-HRU. | 7, 9, 17 |
| Rasio pengiriman (delivery ratio, DR, `dr_ru`) | Fraksi muatan yang sampai ke keluaran unit, dipakai sebagai pengali muatan elemen RU (tidak mengenai air). Nilai `calc`/0 berarti akar rasio waktu konsentrasi HRU terhadap RU, maksimum 1; juga nama jenis objek SWAT+. | 3, 4, 12, 17 |
| Rasio puncak (`pk_rto`) | Pengali debit rata-rata untuk debit puncak harian, ditambah faktor luas 2,66 A^−0,3. | 12 |
| rate | Opsi pelepasan laju tetap (m3/s). | 13 |
| rate\_pct | Opsi pelepasan persentase volume utama per hari. | 13 |
| Reaerasi (`rk2`) | Pemasukan oksigen dari udara sebanding defisit terhadap oksigen jenuh. | 12 |
| Recall (`rec`) | Objek ruang sumber yang memasukkan hidrograf atau beban terukur atau dari model luar (sumber titik, debit terukur) dari file harian, bulanan, tahunan, atau koefisien ekspor; didefinisikan di `recall.rec`. | 3, 4, 5, 12, 13, 17 |
| recall / basin\_psc (tabel keluaran) | Tabel beban titik sumber yang dibaca dari berkas (per objek dan jumlah basin). | 15 |
| Record | Satu baris file dalam istilah Fortran; setiap pernyataan `read` mulai pada record baru. | 4 |
| Region | Basin (`Basin`) atau unit lanskap (`Lsu`), untuk tabel `basin_*` dan `lsunit_*`. | 15 |
| Regresi USGS (`usgs_reg`, `regres`) | Persamaan regresi beban badai urban Driver dan Tasker untuk padatan tersuspensi, N total, dan P total. | 11 |
| Rembesan (seepage, `water_seep`, `seep`) | Air genangan lahan basah atau reservoir yang masuk ke tanah melalui dasarnya, dibatasi konduktivitas dasar (mm/jam); untuk reservoir 240·k·A m3/hari. | 11, 13 |
| Rembesan dalam (deep seepage, `seep`, `deep_seep`) | Bagian resapan yang hilang ke akuifer dalam, menjadi hidrograf kedua akuifer. | 14 |
| Reporter | Pelapor progres yang memancarkan event `progress` setiap kali persentase bulat berubah dan pada hari terakhir. | 15, 16 |
| REQUIRED | Tipe repetisi kolom Parquet tanpa nilai null; tidak memerlukan definition/repetition level. | 15 |
| res\_convert\_mass | Konversi air awal dari fraksi volume dan konsentrasi ke volume dan massa. | 13 |
| Resapan (recharge, `rchrg`) | Air yang masuk ke akuifer dari HRU atau unit routing, dalam mm atas luas akuifer. | 14 |
| Reservoir (`res`) | Objek ruang SWAT+ yang menampung air di balik bendungan dan melepaskannya menurut tabel keputusan; juga dipakai untuk danau. | 13 |
| Residu permukaan (`rsd`, `rsd_tot`) | Bahan organik tanaman mati di permukaan tanah per tanaman (`rsd`) dan total HRU (`rsd_tot`), kg/ha. | 8, 9, 10 |
| Revap (`revap_co`) | Pergerakan air dari akuifer dangkal kembali ke zona tak jenuh, tanaman, atau atmosfer; sebanding PET dan terjadi bila muka air cukup dangkal. | 3, 14 |
| revap\_min (m) | Kedalaman muka air maksimum agar revap terjadi. | 14 |
| rewind | Perintah Fortran untuk kembali ke awal file; dipakai SWAT+ setelah lintasan hitung `imax`. | 4 |
| rndseed\_cond | Benih pembangkit acak khusus yang ditinggalkan `gcycl` untuk kondisi `prob_unif` di tabel keputusan. | 6, 10 |
| Routing lanskap (`rls_routesurf`, `rls_routesoil`) | Pengaliran air dan sedimen dari satu HRU ke HRU lain di hilirnya pada hari yang sama. | 7, 12 |
| Row group | Kelompok baris dalam berkas Parquet; port memakai 2^20 baris per row group. | 15, 16 |
| rpow | Pemanggilan `powf` di port yang meniru `x ** y` gfortran agar hasil identik bit demi bit. | 6, 17 |
| rsdco\_pl | Laju dekomposisi residu tanaman (1/hari) dari `plants.plt`. | 9 |
| rtof | Fraksi bahan organik pupuk yang masuk sebagai residu segar; sisanya ke humus. | 10 |
| ru\_d / ru\_day | Keluaran harian RU yang dicetak; di SWAT+ asli menjumlahkan kelima hidrograf sehingga aliran terhitung dua kali. | 12, 17 |
| run | Perintah CLI yang menjalankan simulasi dan menulis tabel. | 16 |
| run.json | Laporan JSON terakhir dalam arsip tar keluaran wasm: versi, ok, galat, catatan, dan daftar tabel. | 15, 16 |
| Runon | Aliran permukaan, lateral, dan drainase beserta muatannya yang diterima HRU dari HRU lain, dinyatakan per hektar penerima (mm, kg/ha). | 5, 7, 12 |
| RunOptions (`fixes`) | Opsi run engine; `fixes = true` (`--fix-swat`) memperbaiki bug SWAT+ sehingga hasil berbeda dari SWAT+. | 5, 16 |
| RunReport (`hru_skipped`, `routing_skipped`, `channel_skipped`) | Laporan hasil `run_with` tentang bagian simulasi (HRU, perutean, saluran) yang tidak disimulasikan beserta alasannya. | 5, 16, 17 |

### S

| Istilah | Definisi | Bab |
| --- | --- | --- |
| S\_ini | Tampungan pada awal tahun operasi dalam skema Hanasaki. | 13 |
| Salju (`sno_mm`, `snofall`, `snomlt`) | Setara air salju, hujan salju, dan lelehan (mm). | 7 |
| Saluran berumput (grassed waterway, `grassedww.str`) | Saluran dangkal berumput yang mengalirkan limpasan dan mengendapkan sedimen. | 11 |
| Sawah (paddy) | Lahan basah HRU yang hidrologinya (`hydrology.wet`) bernama `paddy`; airnya ditahan ambang dan melimpas di atasnya, luas genangan dianggap seluruh HRU. | 10, 11 |
| SchemaElement | Elemen skema Parquet (tipe fisik, repetisi, nama, converted type). | 15 |
| sdnco | Ambang faktor air tanah di atas mana denitrifikasi terjadi. | 9 |
| sed\_stlr\_co | Koefisien sisa sedimen setelah pengendapan harian, exp(−0,184·d50). | 13 |
| Selisih relatif (r) | \|x-y\| dibagi nilai mutlak yang lebih besar dari keduanya. | 16 |
| Senesens daun (`dlai`, `hu_lai_decl`, `dlai_rate`, `alai_min`) | Penurunan LAI setelah fraksi heat unit `dlai` tercapai; di kasus uji tropis `dlai` disetel 20. | 8, 17 |
| Serasah metabolik (`meta`) | Bagian residu yang mudah terurai. | 9 |
| Serasah struktural (`str`, `lig`, `nonlig`) | Bagian residu yang sulit terurai, terdiri atas lignin dan non-lignin. | 9 |
| Sheet (lembar) | Penyangga teks satu berkas tabel beserta status header dan penulisannya. | 15 |
| sim | Nama sumber di `weather-sta.cli` yang berarti variabel dibangkitkan generator. | 6 |
| Simpanan akuifer (`stor`, mm) | Volume air tersimpan per luas akuifer. | 14 |
| Sinuositas (`sinu`) | Rasio panjang alur terhadap panjang lembah; menentukan jari-jari lengkung tikungan. | 12 |
| Sistem septik (`septic.str`) | Sistem pengolahan air limbah rumah tangga di lokasi yang efluennya meresap ke lapisan tanah melalui bidang resapan; basis datanya dibaca dengan perilaku pergeseran baris yang ditiru persis. | 3, 4, 11 |
| Skema tabel (`Field`, `Kind`) | Deskripsi komponen tipe turunan (nama, jenis, bawaan, panjang larik) yang dipakai port untuk membaca tabel generik. | 3, 4 |
| skip | Operasi penanda akhir tahun rotasi; sisa tahun dilewati sampai `time_control` melangkahinya. | 10 |
| Slot (`file.cio`) | Satu posisi nama file pada sebuah baris `file.cio`; nilai `null` mematikan slot. | 3, 4 |
| sno\_init / sno\_final / snopack | Salju awal, akhir, dan rata-rata periode (mm air). | 5 |
| soil\_awc\_init | Subrutin yang menghitung ulang titik layu, kapasitas lapang, porositas dan air tanah setelah kalibrasi awc/bd/z. | 14 |
| soil\_lyr\_depths.sol | Daftar kedalaman lapisan baku untuk membelah ulang profil tanah; dibuka dengan nama tetap. | 4 |
| soil\_rsd | Residu tanaman di setiap lapisan tanah (terutama dari akar). | 10 |
| soils.sol | Basis data tanah dan lapisannya (kedalaman, BD, AWC, Ksat, karbon, tekstur, dll.). | 4 |
| sol\_p\_model (`soil_p`) | Kode `codes.bsn`; 1 memilih model P dinamis Vadas dan White (`nut_pminrl2`). | 9 |
| soln\_stl\_fr / solp\_stl\_fr | Fraksi kehilangan nutrien terlarut terhadap koefisiennya (0,2). | 13 |
| solradmx (H\_mx) | Radiasi matahari maksimum yang mungkin di permukaan pada hari tertentu (MJ/m²/hari). | 6 |
| Solusi semi-analitis (`wq_semianalyt`) | Solusi eksak reaktor tercampur dengan waktu tinggal, laju orde satu, dan suku sumber tetap. | 12 |
| Sorpsi P (`solpsorb`) | Pengikatan fosfor terlarut oleh partikel tanah, dibatasi kapasitas sorpsi maksimum (`psorpmax`). | 11 |
| sp\_ob | Struktur SWAT+ yang menyimpan jumlah objek spasial per jenis. | 4 |
| Specific yield (hasil jenis, `spyld`, `sp_yld`) | Volume air yang dapat dilepas akuifer per volume akuifer per satuan penurunan muka air; mengubah simpanan (mm) menjadi tebal zona jenuh. | 3, 14 |
| SSP | Rasio P stabil terhadap P aktif pada model P dinamis. | 9 |
| start\_yr / end\_yr | Tahun pertama dan terakhir data objek recall. | 13 |
| Stasiun cuaca (`weather-sta.cli`, `wst`) | Titik yang menggabungkan satu generator dan sumber setiap variabel (gage terukur atau `sim`) untuk dipakai objek spasial. | 6 |
| Station list | File daftar stasiun terukur per variabel (`pcp.cli`, `tmp.cli`, ...), dicari SWAT+ dengan pencarian biner. | 3 |
| `stmp_solt` | Rutin suhu tanah SWAT+. | 7 |
| Stop byte | Bita 0x00 yang menutup struct Thrift compact. | 15 |
| Store | Trait "tambahkan bita ke berkas bernama"; implementasinya `DirStore` (folder di disk) dan `MemStore` (memori). | 15 |
| Stres aerasi (`strsa`, `aeration`) | Pengurangan pertumbuhan saat tanah mendekati jenuh di atas kapasitas lapang. | 8 |
| Stres air (`strsw`, `w_stress`) | Faktor 0–1 pengurang pertumbuhan akibat kekurangan air: rasio serapan air aktual terhadap transpirasi maksimum. | 8, 10 |
| Stres nitrogen (`strsn`) dan stres fosfor (`strsp`) | Faktor 0–1 dari kandungan unsur aktual terhadap optimal (fungsi `nuts`). | 8 |
| Stres salinitas (`strss`) | Faktor stres garam; tidak diporting, selalu 1. | 8 |
| Stres suhu (`strst`) | Faktor 0–1 dari suhu; di SWAT+ selalu sin(1,5707·r). | 8 |
| --strict | Opsi `run` yang membuat program gagal dengan kode keluar 3 bila ada bagian proyek yang tidak disimulasikan; sejak 1.0.1 tabel tetap ditulis lengkap (juga Parquet) lebih dulu. | 16, 17 |
| Sub-langkah Muskingum (`substeps`, `nsteps`) | Pembagian langkah agar Δt ≤ 2K(1−X) (kriteria stabilitas Jeong 2011). | 12 |
| success.fin | Berkas penanda yang ditulis SWAT+ Fortran saat simulasi selesai normal. | 16, 17 |
| Suhu air lokal tertunda | Suhu air dari lahan yang dihitung dari rata-rata suhu udara beberapa hari sebelumnya (2 hari salju, 5 permukaan, 10 lateral, 250 airtanah). | 12 |
| Suhu dasar (`t_base`) | Suhu (°C) di bawahnya tanaman tidak tumbuh. | 8 |
| Suhu kesetimbangan (`t_equil`) | Suhu air yang membuat pertukaran panas permukaan nol; air bergerak ke arahnya sebanding K\_e. | 12 |
| Suhu optimum (`t_opt`) | Suhu (°C) pertumbuhan maksimum; dasar stres suhu. | 8 |
| `surf_bs` | Simpanan lag limpasan permukaan (mm). | 7 |
| `surlag` / `brt` | `surlag` adalah koefisien lag limpasan permukaan basin. `brt` = 1 − exp(−surlag/tconc) adalah fraksi limpasan (atau sedimen) tertampung yang dilepas ke sungai per hari atau per langkah. | 3, 7, 11, 14 |
| sw\_init / sw\_final / sw\_ave | Air tanah awal, akhir, dan rata-rata periode (mm). | 5 |
| SWAT-C (CENTURY) | Model karbon tanah dinamis SWAT berbasis CENTURY/EPIC (Zhang dkk. 2013), aktif bila `cswat` = 2; parameternya dibaca dari `carbon.bsn` dan `carbon_lyr.bsn`. | 3, 9 |
| SWAT-DEG (`sdc`, `chandeg`, `sd_ch`) | Model kanal SWAT+ berbasis geometri trapesium dan kurva rating, dengan degradasi/agradasi dasar dan tebing; dibaca dari `channel-lte.cha` dan `chandeg.con`, tabel keluarannya `channel_sd_*`. | 3, 4, 5, 12 |
| swat-output | Crate Rust yang menulis keluaran engine: tabel teks bergaya SWAT+, Parquet, arsip tar, dan progres JSON-line. | 15 |
| swat-wasm / swat\_run | Modul WebAssembly engine dan ekspornya `swat_run`, yang menjalankan skenario dari arsip tar dan mengembalikan penyangga `[panjang u32 LE][tar]` berisi tabel dan `run.json`. | 15, 16 |
| swat\_alloc | Ekspor modul yang memesan penyangga berukuran tertentu untuk diisi pemanggil. | 16 |
| swat\_free | Ekspor modul yang membebaskan penyangga dari `swat_alloc` atau `swat_run`, dengan panjang yang sama persis. | 16 |

### T

| Istilah | Definisi | Bab |
| --- | --- | --- |
| Tabel generik (`pl_hv_summer1`, `pl_hv_winter1`, `pl_hv_summer2`) | Tabel tanam-panen yang mengambil nama tanaman dari jadwal (`auto_crop`). | 10 |
| Tabel keputusan (decision table, `lum.dtl`, `res_rel.dtl`, `Dtbl`) | Tabel kondisi–alternatif–aksi yang dievaluasi setiap hari untuk memicu operasi pengelolaan lahan (`lum.dtl`) atau menentukan pelepasan air reservoir dan lahan basah (`res_rel.dtl`); aksi berjalan bila salah satu alternatifnya terpenuhi. | 3, 4, 10, 11, 13 |
| Tahanan aerodinamik (r\_a, `rv`) | Hambatan transfer uap dari permukaan ke udara (s/m). | 6 |
| Tahanan kanopi (r\_c, `rc`) | Hambatan stomata kanopi terhadap transpirasi (s/m). | 6 |
| Tahun kabisat Gregorian | Tahun yang habis dibagi 4 dan tidak habis dibagi 100, atau habis dibagi 400; dipakai `time_control`, sementara pembaca cuaca memakai aturan mod 4. | 3, 5 |
| Tahun rotasi (`rot_yr`, `rot_yr_ini`) | Tahun dalam siklus rotasi tanaman; diuji kondisi `year_rot` dan diubah aksi `rot_reset`. | 10 |
| tamp | Setengah selisih rerata suhu maksimum dan minimum bulanan, dipakai menyesuaikan suhu maksimum hari basah/kering. | 6 |
| Tampungan banjir (`e-pv`) | Selisih volume antara pelimpah darurat dan pelimpah utama. | 13 |
| Tampungan mati (*inactive storage*, So) | Volume di bawah outlet yang tidak dapat dilepas. | 13 |
| Tampungan permukaan per langkah (`hhsurf_bs`, `bs_q`, `bs_sed`) | Limpasan dan sedimen yang tertahan di akhir tiap langkah untuk ditunda. | 11 |
| tar (ustar) | Format arsip POSIX berblok 512 bita dengan header berisi nama, mode, ukuran oktal, checksum, dan magic `ustar`; membawa berkas TxtInOut masuk dan tabel keluar modul. | 15, 16 |
| Tegangan geser dasar (bed shear) | Gaya geser aliran permukaan di permukaan tanah (Pa), dasar erosi aliran permukaan. | 11 |
| Tekanan uap jenuh (`ee`, e°) | Tekanan uap air jenuh pada suhu tertentu (kPa). | 6 |
| `temp_gro` | Pemicu pertumbuhan menurut suhu (lawan: menurut kelembapan). | 8 |
| text\_to\_parquet | Fungsi yang mengubah tabel teks SWAT+ menjadi Parquet dengan inferensi tipe dari nama kolom dan 1.000 baris pertama. | 15 |
| TextTables | Implementasi `OutputSink` yang menyusun tabel teks SWAT+ dari hari-hari tercetak dalam penyangga per berkas lalu menuliskannya ke `Store`. | 15, 16 |
| theta\_n / theta\_p (θ) | Koefisien koreksi suhu Arrhenius untuk laju kehilangan N dan P. | 13 |
| Thrift compact protocol | Serialisasi biner ringkas Apache Thrift yang dipakai metadata Parquet. | 15 |
| tillagef | Faktor peningkat dekomposisi akibat pengolahan/pencampuran (`mgt_tillfactor`). | 9 |
| time.sim | File periode simulasi: hari dan tahun awal (`day_start`, `yrc_start`), hari dan tahun akhir (`day_end`, `yrc_end`), serta jumlah langkah waktu per hari (`step`). | 3, 4 |
| `timp` | Faktor lag suhu paket salju. | 7 |
| Tinggi kanopi (`cht`, `chtmx`) | Tinggi kanopi saat ini dan maksimum (m). | 8 |
| Tipe turunan (derived type) | Struktur data Fortran (`type ... end type`) yang komponennya memiliki nilai bawaan; banyak tabel SWAT+ dibaca satu tipe per baris (`read (unit,*) db(i)`). | 3, 4 |
| Titik embun (`dewpt`) | Suhu di mana udara menjadi jenuh; SWAT+ mendekatinya dari suhu rerata dan kelembapan relatif. | 6 |
| Titik layu (`wp`, `wpmm`) | Kadar air di mana tanaman tidak dapat lagi menyerap air (fraksi volume, mm). | 7 |
| titldum / header | Variabel boneka SWAT+ untuk membaca dan membuang baris judul dan kepala kolom. | 4 |
| tmpf | Kode fungsi faktor suhu dekomposisi model karbon (1, 2 fungsi beta, 3 SWAT). | 9 |
| Toleransi K12 | Aturan kesetaraan proyek (keputusan desain RustGIS) untuk validasi: selisih relatif 1e-4 atau satu satuan pada desimal terakhir yang dicetak. | 16, 17 |
| tpco | Konsentrasi total P air reservoir (dihitung untuk menetapkan klorofil-a semu). | 13 |
| Transpirasi maksimum (`ep_max`, Et) | Transpirasi tanaman maksimum (potensial) harian HRU (mm), dibagi ke tanaman menurut LAI. | 6, 7, 8 |
| tsin | Inflow per langkah sebuah objek, jumlah dari hidrograf per langkah para pengirimnya. | 12 |
| Turnover daun (`leaf_tov`) | Fraksi massa daun yang gugur per hari pada senesens perennial. | 8 |
| TxtInOut | Folder proyek atau skenario SWAT+ berisi `file.cio` dan semua file masukan teks, tempat keluaran ditulis; dibaca port melalui `FileSource`. | 3, 4, 16 |
| typeflag | Bita tipe entri tar (`'0'` berkas biasa, `'5'` folder, `'L'` nama panjang GNU, `'x'` pax). | 15 |

### U

| Istilah | Definisi | Bab |
| --- | --- | --- |
| ULP (unit in the last place) | Jarak antara dua bilangan floating-point bertetangga, yaitu satu satuan bit terakhir representasinya. | 15, 16, 17 |
| Unit lanskap (landscape unit, LSU, `ls_unit`) | Pengelompokan HRU untuk agregasi keluaran (tabel `lsunit_*`), dengan bobot `ru_frac`. | 3, 4, 5 |
| Unit routing (routing unit, RU, `rout_unit`) | Kelompok HRU (dan elemen lain) yang hidrografnya dijumlahkan menjadi satu objek sebelum diteruskan ke kanal atau akuifer. | 3, 4, 5, 12 |
| Urutan perintah (`cmd_next`, `hru_order`) | Urutan eksekusi objek SWAT+: sebuah objek berjalan setelah semua pengirimnya (dan, bagi RU, semua elemennya), sehingga penerima air dihitung setelah pemberinya. | 5, 12 |
| USLE | Universal Soil Loss Equation; `usle` adalah erosi potensial yang dilaporkan. | 9 |
| USLE K (`usle_k`) | Faktor erodibilitas tanah dalam persamaan USLE/MUSLE. | 3 |
| `usle_mult` | Hasil kali faktor USLE K, P, LS, faktor fragmen kasar (CFRG), dan 11,8 per HRU, dipakai MUSLE. | 7, 9, 14 |

### V

| Istilah | Definisi | Bab |
| --- | --- | --- |
| validate.yml | Alur kerja CI yang menjalankan SWAT+ Fortran dan port pada 35 skenario lalu membandingkan semua tabel. | 17 |
| Variable storage coefficient (VSC, `rte` 0, `scoef`) | Routing dengan outflow = SC·V, SC = 2Δt/(2TT + Δt). | 12 |
| Varian | Skenario uji turunan proyek nyata yang menghapus atau menambah satu fitur (Ames\_*, Bh\_*, Ps\_\*). | 17 |
| Varint (ULEB128) | Bilangan tak bertanda 7 bit per bita dengan bit 7 sebagai penanda lanjutan. | 15 |
| Volatilisasi (`rvol`) | Hilangnya amonium sebagai gas amonia ke atmosfer, bergantung suhu dan kedalaman lapisan. | 9 |
| Volume darurat (`evol`) | Volume tampungan pada kedalaman pelimpah darurat (m3); minimal 1,2 kali volume utama bila ada ambang. | 11 |
| Volume utama (`pvol`) | Volume tampungan lahan basah pada kedalaman utama atau tinggi ambang (m3). | 11 |

### W

| Istilah | Definisi | Bab |
| --- | --- | --- |
| Waktu dasar / waktu puncak (`tb`, `tp`) | Durasi unit hidrograf (0,5 + 0,6 tc + adj, maks 48 jam) dan waktu puncaknya (3/8 tb). | 12 |
| Waktu konsentrasi (`tconc`, `ru_tc`) | Waktu tempuh limpasan dari titik terjauh ke keluaran HRU (jam), jumlah waktu aliran darat dan aliran saluran; untuk RU dijumlahkan dari aliran permukaan dan aliran alur. | 7, 9, 12, 14 |
| Waktu paruh nitrat (`hlife_n`, hari) | Parameter peluruhan orde satu nitrat di akuifer; dipakai sebagai `nloss = exp(-0.693/(hlife_n+0.1))`. | 14 |
| Waktu paruh penumpukan (`thalf`) | Hari yang dibutuhkan untuk mencapai setengah padatan maksimum. | 11 |
| Waktu sejak pencucian (`twash`) | Lama akumulasi padatan setara di permukaan kedap air sejak pencucian terakhir (hari). | 7, 11 |
| Waktu tempuh (`ttime`, `rttime`, jam) | Panjang saluran dibagi kecepatan, L/(3,6 v). | 12 |
| Waktu tempuh perkolasi (`hk`, TT) | (ul − fc)/Ksat dalam jam, minimal 1; mengatur bagian air berlebih lapisan yang turun per hari. | 7, 14 |
| wash\_bed\_fr | Fraksi erosi tebing yang langsung mengendap di dasar saluran. | 12 |
| watf | Kode fungsi faktor air dekomposisi model karbon (1 SWAT, 2 air-aerasi). | 9 |
| `wavp` | Laju penurunan RUE per kPa VPD di atas 1 kPa. | 8 |
| WbPrm (`wbody_prm`, `water_body_data_parameters`) | Parameter sedimen dan nutrien badan air. | 13 |
| Web Worker | Thread latar peramban tempat modul dijalankan agar simulasi sinkron tidak membekukan antarmuka. | 16 |
| WebAssembly (WASM) | Format biner portabel untuk mesin virtual bertumpuk yang dijalankan peramban dan Node; engine dikompilasi ke target `wasm32-unknown-unknown`. | 16 |
| weir (opsi pelepasan reservoir) | Opsi pelepasan lewat bendung (persamaan ambang), tidak didukung port untuk reservoir. | 13 |
| wgncur / wgnold | Tiga deviat berkorelasi hari ini dan kemarin untuk suhu maksimum, suhu minimum, dan radiasi. | 6 |
| wq\_k2m | Konversi proses orde satu menjadi suku sumber konstan setara. | 12 |
| `wsyf` | Batas bawah indeks panen pada stres air; di port hanya dipakai panen biji untuk menaikkan `harv_idx`. | 8 |

### X

| Istilah | Definisi | Bab |
| --- | --- | --- |
| xmon | Subrutin SWAT+ yang mengubah hari ke-n tahun (hari Julian) menjadi bulan dan tanggal menurut `ndays`. | 3, 5 |

### Y

| Istilah | Definisi | Bab |
| --- | --- | --- |
| yr\_skip | Bendera bahwa jadwal HRU sedang menunggu tahun baru setelah `skip`. | 10 |
| yrc | Tahun kalender berjalan. | 5 |
| yrs | Tahun simulasi ke-n (1 = pertama). | 5 |
| yrs\_prt | Jumlah tahun tercetak (hari tercetak dibagi 365) yang menjadi pembagi fluks rata-rata tahunan. | 5, 17 |

### Z

| Istilah | Definisi | Bab |
| --- | --- | --- |
| Zigzag | Pemetaan bilangan bertanda ke tak bertanda: 0→0, −1→1, 1→2, −2→3, ... | 15 |
| `zmx` | Kedalaman perakaran maksimum profil tanah (mm), dipangkas permanen oleh operasi tanam. | 8, 10 |
