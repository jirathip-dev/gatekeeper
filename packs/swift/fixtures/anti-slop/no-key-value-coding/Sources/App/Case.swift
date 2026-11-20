import Foundation

func name(of owner: NSObject) -> Bool {
    owner.value(forKey: "name") != nil
}
