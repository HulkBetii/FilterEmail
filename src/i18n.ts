export type Language = "en" | "vi";

export type ErrorPayload = {
  message_en: string;
  message_vi: string;
};

type TranslationShape = {
  idleBanner: string;
  progressBanner: (processedLines: number, currentDomain?: string | null) => string;
  completeBanner: string;
  selectedFileBanner: (name: string) => string;
  selectedOutputBanner: string;
  preparingBanner: string;
  labels: {
    invalid: string;
    public: string;
    edu: string;
    targeted: string;
    custom: string;
    duplicates: string;
    mx_dead: string;
    mx_has_mx: string;
    mx_a_fallback: string;
    mx_inconclusive: string;
    mx_parked: string;
    mx_disposable: string;
    mx_typo: string;
    smtp_deliverable: string;
    smtp_rejected: string;
    smtp_catchall: string;
    smtp_unknown: string;
    final_alive: string;
    final_dead: string;
    final_unknown: string;
    smtp_attempted_emails: string;
    smtp_cache_hits: string;
    smtp_coverage_percent: string;
    smtp_policy_blocked: string;
    smtp_temp_failure: string;
    smtp_mailbox_full: string;
    smtp_mailbox_disabled: string;
    smtp_bad_mailbox: string;
    smtp_bad_domain: string;
    smtp_network_error: string;
    smtp_protocol_error: string;
    smtp_timeout: string;
    smtp_unknown_breakdown: string;
    smtp_alive_note: string;
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
    targetedInputLabel: string;
    targetedInputPlaceholder: string;
    mxCheckLabel: string;
    timeoutLabel: string;
    timeoutHint: string;
    concurrencyLabel: string;
    concurrencyHint: string;
    persistentCacheLabel: string;
    persistentCacheHint: string;
    smtpVerifyLabel: string;
    smtpVerifyHint: string;
    vpsApiUrlLabel: string;
    vpsApiUrlPlaceholder: string;
    vpsApiKeyLabel: string;
    vpsApiKeyPlaceholder: string;
    cacheStatus: (hits: number) => string;
    cacheCoverage: (hits: number, total: number) => string;
    reviewNote: string;
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
    verifySummaryTitle: string;
    verifySummaryBody: string;
    summaryTotal: string;
    summaryFolder: string;
    summaryVerified: string;
    summaryCacheHits: string;
    summaryInvalidRate: string;
    summaryPublicRate: string;
    summaryEduRate: string;
    summaryTargetedRate: string;
    summaryCustomRate: string;
    summaryDeadRate: string;
    summaryReviewRate: string;
    summaryFallbackRate: string;
    summaryParkedRate: string;
    summaryDisposableRate: string;
    summaryTypoRate: string;
    openHistory: string;
    historyTitle: string;
    historySuccessGroup: string;
    historyReviewGroup: string;
    historyFailureGroup: string;
    historySmtpGroup: string;
    clearHistory: string;
    emptyHistory: string;
    close: string;
    total: string;
    valid: string;
    deadDomains: string;
    reviewDomains: string;
    smtpSummaryTitle: string;
    smtpSummaryBody: string;
    smtpChecked: string;
    smtpElapsed: string;
    tabBasicFilter: string;
    tabDnsVerify: string;
  };
};

export const translations = {
  en: {
    idleBanner:
      "Drop a .txt or .csv file, choose an output folder, and start processing.",
    progressBanner: (processedLines: number, currentDomain?: string | null) =>
      currentDomain
        ? `Deep scanning ${processedLines.toLocaleString("en-US")} lines. Current domain: ${currentDomain}`
        : `Streaming ${processedLines.toLocaleString("en-US")} lines without loading the whole file into memory.`,
    completeBanner:
      "Processing complete. Result files are ready in the selected folder.",
    selectedFileBanner: (name: string) =>
      `Selected ${name}. Choose an output folder when you’re ready.`,
    selectedOutputBanner:
      "Output folder selected. You can start processing whenever you’re ready.",
    preparingBanner: "Preparing stream processor, DNS resolver, and output writers...",
    labels: {
      invalid: "Invalid",
      public: "Public Mail",
      edu: "Edu / Gov",
      targeted: "Targeted",
      custom: "Other",
      duplicates: "Duplicates",
      mx_dead: "Dead Domains",
      mx_has_mx: "Valid MX",
      mx_a_fallback: "A Record Fallback",
      mx_inconclusive: "Needs Review",
      mx_parked: "Parked",
      mx_disposable: "Disposable",
      mx_typo: "Typos",
      smtp_deliverable: "SMTP Deliverable",
      smtp_rejected: "SMTP Rejected",
      smtp_catchall: "SMTP Catch-All",
      smtp_unknown: "SMTP Unknown",
      final_alive: "Final Alive",
      final_dead: "Final Dead",
      final_unknown: "Final Unknown",
      smtp_attempted_emails: "SMTP Attempted",
      smtp_cache_hits: "SMTP Cache Hits",
      smtp_coverage_percent: "SMTP Coverage",
      smtp_policy_blocked: "Policy Blocked",
      smtp_temp_failure: "Temp Failure",
      smtp_mailbox_full: "Mailbox Full",
      smtp_mailbox_disabled: "Mailbox Disabled",
      smtp_bad_mailbox: "Bad Mailbox",
      smtp_bad_domain: "Bad Domain",
      smtp_network_error: "Network Error",
      smtp_protocol_error: "Protocol Error",
      smtp_timeout: "SMTP Timeout",
      smtp_unknown_breakdown: "Unknown Breakdown",
      smtp_alive_note:
        "Alive means high-confidence SMTP acceptance for this exact email address. It is not a guarantee of inbox placement.",
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
      targetedInputLabel: "Targeted Domains (Optional)",
      targetedInputPlaceholder: "e.g. vnpt.vn, fpt.com",
      mxCheckLabel: "Enable Deep DNS Scan",
      timeoutLabel: "DNS Timeout (ms)",
      timeoutHint: "Recommended 1500ms. Lower is faster, higher is more tolerant of slow DNS.",
      concurrencyLabel: "Max Concurrent Lookups",
      concurrencyHint: "Recommended 30-50 to balance speed and resolver pressure.",
      persistentCacheLabel: "Persistent DNS Cache",
      persistentCacheHint: "Save DNS verification results in SQLite for 6 hours to speed up repeated scans.",
      smtpVerifyLabel: "SMTP Verify (VPS)",
      smtpVerifyHint:
        "After DNS finishes, send only Has MX domains to the VPS SMTP probe service for RCPT-only verification.",
      vpsApiUrlLabel: "VPS API URL",
      vpsApiUrlPlaceholder: "https://your-vps.example.com",
      vpsApiKeyLabel: "VPS API Key",
      vpsApiKeyPlaceholder: "Bearer key for /verify/smtp",
      cacheStatus: (hits: number) => `SQLite cache enabled • TTL 6h • ${hits.toLocaleString("en-US")} cache hit(s)`,
      cacheCoverage: (hits: number, total: number) =>
        `${hits.toLocaleString("en-US")}/${total.toLocaleString("en-US")} domains served from cache`,
      reviewNote: "Inconclusive domains are kept in a separate review bucket and never treated as dead.",
      start: "Start Processing",
      processing: "Filtering...",
      openFolder: "Open Folder",
      classified: "Classified Records",
      classifiedBody:
        "The Rust backend processes files line by line with read and write buffers, keeping memory usage stable even for massive lists.",
      heroBadge: "Tauri v2 + Rust stream processing",
      heroTitle: "Sort massive email lists without touching your RAM ceiling.",
      heroBody:
        "Filter `.txt` and `.csv` files line by line into Invalid, Public, Edu/Gov, Targeted, Other, and DNS review buckets with live progress and desktop-native file handling.",
      language: "Language",
      english: "English",
      vietnamese: "Tiếng Việt",
      emailFilter: "Email Lists",
      genericBackendError:
        "An unexpected backend error occurred while processing the file.",
      summaryTitle: "Final Summary",
      summaryBody:
        "Processing finished successfully. Review the DNS buckets before opening the result folder.",
      verifySummaryTitle: "DNS Verification Summary",
      verifySummaryBody:
        "Verification finished. Review the success, failure, and review-only DNS buckets before using the output files.",
      summaryTotal: "Total Records",
      summaryFolder: "Result Folder",
      summaryVerified: "Verified Deliverable",
      summaryCacheHits: "Cache Hits",
      summaryInvalidRate: "Invalid Rate",
      summaryPublicRate: "Public Rate",
      summaryEduRate: "Edu / Gov Rate",
      summaryTargetedRate: "Targeted Rate",
      summaryCustomRate: "Other Rate",
      summaryDeadRate: "Dead Rate",
      summaryReviewRate: "Review Rate",
      summaryFallbackRate: "A Fallback Rate",
      summaryParkedRate: "Parked Rate",
      summaryDisposableRate: "Disposable Rate",
      summaryTypoRate: "Typo Rate",
      openHistory: "History",
      historyTitle: "Processing History",
      historySuccessGroup: "Success",
      historyReviewGroup: "Review",
      historyFailureGroup: "Failure",
      historySmtpGroup: "SMTP",
      clearHistory: "Clear History",
      emptyHistory: "No history records yet.",
      close: "Close",
      total: "Total",
      valid: "Valid",
      deadDomains: "Dead",
      reviewDomains: "Review",
      smtpSummaryTitle: "SMTP Verification",
      smtpSummaryBody:
        "This layer runs only for domains that already passed DNS with a valid MX record.",
      smtpChecked: "SMTP Checked",
      smtpElapsed: "SMTP Time",
      tabBasicFilter: "Basic Filter",
      tabDnsVerify: "Verify DNS",
    },
  },
  vi: {
    idleBanner:
      "Thả tệp .txt hoặc .csv, chọn thư mục đầu ra, rồi bắt đầu xử lý.",
    progressBanner: (processedLines: number, currentDomain?: string | null) =>
      currentDomain
        ? `Đang quét sâu ${processedLines.toLocaleString("vi-VN")} dòng. Domain hiện tại: ${currentDomain}`
        : `Đang xử lý luồng ${processedLines.toLocaleString("vi-VN")} dòng mà không nạp toàn bộ tệp vào RAM.`,
    completeBanner:
      "Xử lý hoàn tất. Các tệp kết quả đã sẵn sàng trong thư mục đã chọn.",
    selectedFileBanner: (name: string) =>
      `Đã chọn ${name}. Hãy chọn thư mục đầu ra khi bạn sẵn sàng.`,
    selectedOutputBanner:
      "Đã chọn thư mục đầu ra. Bạn có thể bắt đầu xử lý bất cứ lúc nào.",
    preparingBanner: "Đang chuẩn bị bộ xử lý luồng, DNS resolver và các bộ ghi đầu ra...",
    labels: {
      invalid: "Không hợp lệ",
      public: "Mail công cộng",
      edu: "Giáo dục / Chính phủ",
      targeted: "Tùy chọn",
      custom: "Khác",
      duplicates: "Bị trùng",
      mx_dead: "Domain chết",
      mx_has_mx: "MX hợp lệ",
      mx_a_fallback: "Fallback A Record",
      mx_inconclusive: "Cần review",
      mx_parked: "Parked",
      mx_disposable: "Disposable",
      mx_typo: "Sai chính tả",
      smtp_deliverable: "SMTP Có Thể Gửi",
      smtp_rejected: "SMTP Từ Chối",
      smtp_catchall: "SMTP Catch-All",
      smtp_unknown: "SMTP Chưa Rõ",
      final_alive: "Kết quả Alive",
      final_dead: "Kết quả Dead",
      final_unknown: "Kết quả Unknown",
      smtp_attempted_emails: "SMTP Đã Thử",
      smtp_cache_hits: "SMTP Cache Hits",
      smtp_coverage_percent: "Độ Phủ SMTP",
      smtp_policy_blocked: "Bị Chặn Chính Sách",
      smtp_temp_failure: "Lỗi Tạm Thời",
      smtp_mailbox_full: "Hộp Thư Đầy",
      smtp_mailbox_disabled: "Hộp Thư Bị Vô Hiệu",
      smtp_bad_mailbox: "Mailbox Không Tồn Tại",
      smtp_bad_domain: "Domain Không Tồn Tại",
      smtp_network_error: "Lỗi Mạng",
      smtp_protocol_error: "Lỗi Giao Thức",
      smtp_timeout: "SMTP Timeout",
      smtp_unknown_breakdown: "Chi Tiết Unknown",
      smtp_alive_note:
        "Alive nghĩa là SMTP chấp nhận chính xác địa chỉ email này với độ tin cậy cao. Đây không phải cam kết email sẽ vào inbox.",
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
      targetedInputLabel: "Đuôi mail tùy chọn",
      targetedInputPlaceholder: "vd: vnpt.vn, fpt.com",
      mxCheckLabel: "Bật Deep DNS Scan",
      timeoutLabel: "Timeout DNS (ms)",
      timeoutHint: "Khuyên dùng 1500ms. Thấp hơn sẽ nhanh hơn, cao hơn sẽ chịu lỗi DNS chậm tốt hơn.",
      concurrencyLabel: "Số lookup đồng thời tối đa",
      concurrencyHint: "Khuyên dùng 30-50 để cân bằng tốc độ và áp lực lên resolver.",
      persistentCacheLabel: "Persistent DNS Cache",
      persistentCacheHint: "Lưu kết quả xác minh DNS vào SQLite trong 6 giờ để tăng tốc các lần quét lặp lại.",
      smtpVerifyLabel: "SMTP Verify (VPS)",
      smtpVerifyHint:
        "Sau khi DNS xong, chỉ các domain Has MX mới được gửi tới VPS để probe RCPT mà không gửi email thật.",
      vpsApiUrlLabel: "VPS API URL",
      vpsApiUrlPlaceholder: "https://your-vps.example.com",
      vpsApiKeyLabel: "VPS API Key",
      vpsApiKeyPlaceholder: "Bearer key cho /verify/smtp",
      cacheStatus: (hits: number) => `SQLite cache đang bật • TTL 6 giờ • ${hits.toLocaleString("vi-VN")} cache hit`,
      cacheCoverage: (hits: number, total: number) =>
        `${hits.toLocaleString("vi-VN")}/${total.toLocaleString("vi-VN")} domain lấy từ cache`,
      reviewNote: "Những domain Inconclusive luôn được đưa vào bucket review riêng, không bị xem là mail chết.",
      start: "Bắt Đầu Xử Lý",
      processing: "Đang lọc...",
      openFolder: "Mở Kết Quả",
      classified: "Tổng Bản Ghi Đã Phân Loại",
      classifiedBody:
        "Phần backend Rust xử lý từng dòng với bộ đệm đọc và ghi, nên mức dùng bộ nhớ vẫn ổn định ngay cả với các tệp rất lớn.",
      heroBadge: "Tauri v2 + xử lý luồng bằng Rust",
      heroTitle: "Phân loại danh sách email cực lớn mà không chạm trần RAM.",
      heroBody:
        "Lọc tệp `.txt` và `.csv` theo từng dòng vào các nhóm Không hợp lệ, Công cộng, Giáo dục/Chính phủ, Tùy chọn, Khác và các bucket review DNS với tiến độ trực tiếp.",
      language: "Ngôn ngữ",
      english: "English",
      vietnamese: "Tiếng Việt",
      emailFilter: "Danh sách email",
      genericBackendError:
        "Đã xảy ra lỗi backend ngoài dự kiến trong lúc xử lý tệp.",
      summaryTitle: "Tổng Kết Cuối Cùng",
      summaryBody:
        "Quá trình xử lý đã hoàn tất. Hãy kiểm tra các bucket DNS trước khi mở thư mục kết quả.",
      verifySummaryTitle: "Tổng Kết Xác Minh DNS",
      verifySummaryBody:
        "Quá trình xác minh đã hoàn tất. Hãy kiểm tra các bucket DNS thành công, lỗi và cần review trước khi dùng tệp kết quả.",
      summaryTotal: "Tổng Bản Ghi",
      summaryFolder: "Thư Mục Kết Quả",
      summaryVerified: "Có Thể Gửi",
      summaryCacheHits: "Cache Hit",
      summaryInvalidRate: "Tỷ Lệ Không Hợp Lệ",
      summaryPublicRate: "Tỷ Lệ Công Cộng",
      summaryEduRate: "Tỷ Lệ Giáo Dục / Chính Phủ",
      summaryTargetedRate: "Tỷ Lệ Tùy Chọn",
      summaryCustomRate: "Tỷ Lệ Khác",
      summaryDeadRate: "Tỷ Lệ Domain Chết",
      summaryReviewRate: "Tỷ Lệ Cần Review",
      summaryFallbackRate: "Tỷ Lệ A Fallback",
      summaryParkedRate: "Tỷ Lệ Parked",
      summaryDisposableRate: "Tỷ Lệ Disposable",
      summaryTypoRate: "Tỷ Lệ Sai Chính Tả",
      openHistory: "Lịch sử",
      historyTitle: "Lịch Sử Phiên Lọc",
      historySuccessGroup: "Thành công",
      historyReviewGroup: "Cần review",
      historyFailureGroup: "Lỗi",
      historySmtpGroup: "SMTP",
      clearHistory: "Xóa Lịch Sử",
      emptyHistory: "Chưa có lưu trữ nào.",
      close: "Đóng",
      total: "Tổng",
      valid: "Hợp lệ",
      deadDomains: "Chết",
      reviewDomains: "Review",
      smtpSummaryTitle: "Xác Minh SMTP",
      smtpSummaryBody:
        "Lớp này chỉ chạy cho các domain đã vượt qua DNS với MX hợp lệ.",
      smtpChecked: "Đã Kiểm SMTP",
      smtpElapsed: "Thời Gian SMTP",
      tabBasicFilter: "Lọc Thông Thường",
      tabDnsVerify: "Xác Minh DNS",
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
