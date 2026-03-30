(module App.Main)
(import App.CompilerMode)
(import App.PipelineSmoke)
(defn main [] (if (> (string-length (command-line-arg 1)) 0) (compile-file-mode) (run-main-smoke)))
