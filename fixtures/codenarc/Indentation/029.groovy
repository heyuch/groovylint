
class MyClass {
    private DirectoryResults processDirectory(String dir, RuleSet ruleSet) {
        def dirResults = new DirectoryResults(dir)
        def dirFile = new File((String) baseDirectory, (String) dir)
        dirFile.eachFile { file ->
            def dirPrefix = dir ? dir + SEP : dir
            def filePath = dirPrefix + file.name
            if (file.directory) {
                def subdirResults = processDirectory(filePath, ruleSet)
                // If any of the descendent directories have matching files, then include in final results
                if (subdirResults.getTotalNumberOfFiles(true)) {
                    dirResults.addChild(subdirResults)
                }
            }
            else {
                processFile(filePath, dirResults, ruleSet)
            }
        }
        dirResults
    }
}
        