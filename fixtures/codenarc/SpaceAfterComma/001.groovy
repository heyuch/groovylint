
            class MyClass {
                def template() {
                    return {
                        td(getResourceBundleString('htmlReport.summary.allPackages'), class:'allPackages')
                        td {
                            a(pathName, href:"#${pathName}")
                        }
                    }
                }
            }
        