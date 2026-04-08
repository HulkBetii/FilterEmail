export type Language = "en" | "vi";

export type ErrorPayload = {
  message_en: string;
  message_vi: string;
};

type TranslationShape = {
  idleBanner: string;
  progressBanner: (processedLines: number) => string;
  completeBanner: string;
  selectedFileBanner: (name: string) => string;
  selectedOutputBanner: string;
  preparingBanner: string;
  labels: {
    invalid: string;
    public: string;
    edu: string;
    custom: string;
    progress: string;
    linesProcessed: string;
    elapsed: string;
    dragTitle: string;
    dragBody: string;
    chooseFile: string;
    selectedFile: string;
    noFile: string;
    outputFolder: string;
    noFolder: string;
    selectFolder: string;
    start: string;
    processing: string;
    openFolder: string;
    classified: string;
    classifiedBody: string;
    heroBadge: string;
    heroTitle: string;
    heroBody: string;
    language: string;
    english: string;
    vietnamese: string;
    emailFilter: string;
    genericBackendError: string;
    summaryTitle: string;
    summaryBody: string;
    summaryTotal: string;
    summaryFolder: string;
    summaryInvalidRate: string;
    summaryPublicRate: string;
    summaryEduRate: string;
    summaryCustomRate: string;
  };
};

export const translations = {
  en: {
    idleBanner:
      "Drop a .txt or .csv file, choose an output folder, and start processing.",
    progressBanner: (processedLines: number) =>
      `Streaming ${processedLines.toLocaleString("en-US")} lines without loading the whole file into memory.`,
    completeBanner:
      "Processing complete. Result files are ready in the selected folder.",
    selectedFileBanner: (name: string) =>
      `Selected ${name}. Choose an output folder when you’re ready.`,
    selectedOutputBanner:
      "Output folder selected. You can start processing whenever you’re ready.",
    preparingBanner: "Preparing Rust stream processor and output writers...",
    labels: {
      invalid: "Invalid",
      public: "Public Mail",
      edu: "Edu / Gov",
      custom: "Custom",
      progress: "Progress",
      linesProcessed: "Lines Processed",
      elapsed: "Time Elapsed",
      dragTitle: "Drag and drop your source file",
      dragBody:
        "Drop a `.txt` or `.csv` email list anywhere on the window, or use the file picker below for a more deliberate flow.",
      chooseFile: "Choose Source File",
      selectedFile: "Selected file",
      noFile: "No file selected yet",
      outputFolder: "Output folder",
      noFolder: "Choose where result files should be written",
      selectFolder: "Select Folder",
      start: "Start Processing",
      processing: "Processing...",
      openFolder: "Open Result Folder",
      classified: "Classified Records",
      classifiedBody:
        "The Rust backend streams one line at a time with buffered reads and writes, so memory usage stays predictable even on very large files.",
      heroBadge: "Tauri v2 + Rust stream processing",
      heroTitle: "Sort massive email lists without touching your RAM ceiling.",
      heroBody:
        "Filter `.txt` and `.csv` files line by line into Invalid, Public, Edu/Gov, and Custom buckets with live progress and desktop-native file handling.",
      language: "Language",
      english: "English",
      vietnamese: "Tiếng Việt",
      emailFilter: "Email Lists",
      genericBackendError:
        "An unexpected backend error occurred while processing the file.",
      summaryTitle: "Final Summary",
      summaryBody:
        "Processing finished successfully. Review the final totals and distribution before opening the result folder.",
      summaryTotal: "Total Records",
      summaryFolder: "Result Folder",
      summaryInvalidRate: "Invalid Rate",
      summaryPublicRate: "Public Rate",
      summaryEduRate: "Edu / Gov Rate",
      summaryCustomRate: "Custom Rate",
    },
  },
  vi: {
    idleBanner:
      "Thả tệp .txt hoặc .csv, chọn thư mục đầu ra, rồi bắt đầu xử lý.",
    progressBanner: (processedLines: number) =>
      `Đang xử lý luồng ${processedLines.toLocaleString("vi-VN")} dòng mà không nạp toàn bộ tệp vào RAM.`,
    completeBanner:
      "Xử lý hoàn tất. Các tệp kết quả đã sẵn sàng trong thư mục đã chọn.",
    selectedFileBanner: (name: string) =>
      `Đã chọn ${name}. Hãy chọn thư mục đầu ra khi bạn sẵn sàng.`,
    selectedOutputBanner:
      "Đã chọn thư mục đầu ra. Bạn có thể bắt đầu xử lý bất cứ lúc nào.",
    preparingBanner: "Đang chuẩn bị bộ xử lý luồng Rust và các bộ ghi đầu ra...",
    labels: {
      invalid: "Không hợp lệ",
      public: "Mail công cộng",
      edu: "Giáo dục / Chính phủ",
      custom: "Doanh nghiệp",
      progress: "Tiến độ",
      linesProcessed: "Số dòng đã xử lý",
      elapsed: "Thời gian đã trôi qua",
      dragTitle: "Kéo và thả tệp nguồn của bạn",
      dragBody:
        "Thả danh sách email `.txt` hoặc `.csv` ở bất kỳ đâu trong cửa sổ, hoặc dùng trình chọn tệp bên dưới để thao tác chính xác hơn.",
      chooseFile: "Chọn Tệp Nguồn",
      selectedFile: "Tệp đã chọn",
      noFile: "Chưa chọn tệp nào",
      outputFolder: "Thư mục đầu ra",
      noFolder: "Chọn nơi sẽ ghi các tệp kết quả",
      selectFolder: "Chọn Thư Mục",
      start: "Bắt Đầu Xử Lý",
      processing: "Đang Xử Lý...",
      openFolder: "Mở Thư Mục Kết Quả",
      classified: "Tổng Bản Ghi Đã Phân Loại",
      classifiedBody:
        "Phần backend Rust xử lý từng dòng với bộ đệm đọc và ghi, nên mức dùng bộ nhớ vẫn ổn định ngay cả với các tệp rất lớn.",
      heroBadge: "Tauri v2 + xử lý luồng bằng Rust",
      heroTitle: "Phân loại danh sách email cực lớn mà không chạm trần RAM.",
      heroBody:
        "Lọc tệp `.txt` và `.csv` theo từng dòng vào 4 nhóm Không hợp lệ, Công cộng, Giáo dục/Chính phủ và Doanh nghiệp với tiến độ trực tiếp cùng khả năng xử lý tệp native trên desktop.",
      language: "Ngôn ngữ",
      english: "English",
      vietnamese: "Tiếng Việt",
      emailFilter: "Danh sách email",
      genericBackendError:
        "Đã xảy ra lỗi backend ngoài dự kiến trong lúc xử lý tệp.",
      summaryTitle: "Tổng Kết Cuối Cùng",
      summaryBody:
        "Quá trình xử lý đã hoàn tất thành công. Hãy xem tổng số và phân bố cuối cùng trước khi mở thư mục kết quả.",
      summaryTotal: "Tổng Bản Ghi",
      summaryFolder: "Thư Mục Kết Quả",
      summaryInvalidRate: "Tỷ Lệ Không Hợp Lệ",
      summaryPublicRate: "Tỷ Lệ Công Cộng",
      summaryEduRate: "Tỷ Lệ Giáo Dục / Chính Phủ",
      summaryCustomRate: "Tỷ Lệ Doanh Nghiệp",
    },
  },
} satisfies Record<Language, TranslationShape>;

export function formatBackendError(payload: ErrorPayload, language: Language) {
  return language === "vi" ? payload.message_vi : payload.message_en;
}

export function getSavedLanguage(): Language {
  const saved = window.localStorage.getItem("filteremail-language");
  return saved === "vi" ? "vi" : "en";
}

export function persistLanguage(language: Language) {
  window.localStorage.setItem("filteremail-language", language);
}
