(module App.Main)
(import App.CompilerMode)
(import App.PipelineSmoke)
(defn main [] (if (> (string-length (command-line-arg 1)) 0) (if (> (string-length (command-line-arg 7)) 0) (compile-file-mode-token-debug) (if (> (string-length (command-line-arg 6)) 0) (compile-file-mode-ir-debug) (if (> (string-length (command-line-arg 5)) 0) (compile-file-mode-build-progress-debug) (if (> (string-length (command-line-arg 4)) 0) (compile-file-mode-progress-debug) (if (> (string-length (command-line-arg 3)) 0) (compile-file-mode-path-debug) (if (> (string-length (command-line-arg 2)) 0) (compile-file-mode-debug) (compile-file-mode))))))) (run-main-smoke)))
