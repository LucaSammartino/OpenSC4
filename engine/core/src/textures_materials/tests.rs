// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[derive(Clone)]
struct Image {
    description: ImageDescription,
    data: Vec<Vec<Vec<u8>>>,
}

impl Image {
    fn new(width: u32, height: u32, levels: u32, frames: u32, format: u32) -> Self {
        Self {
            description: ImageDescription {
                width,
                height,
                levels,
                frames,
                format,
                component_type: 1,
                bits: BitCounts {
                    red: 8,
                    green: 8,
                    blue: 8,
                    alpha: 8,
                },
            },
            data: (0..frames)
                .map(|frame| (0..levels).map(|level| vec![(frame * 16 + level) as u8]).collect())
                .collect(),
        }
    }
}

impl ImageInfo for Image {
    fn description(&self) -> ImageDescription {
        self.description
    }
    fn level_data(&self, frame: u32, level: u32) -> Option<&[u8]> {
        self.data.get(frame as usize)?.get(level as usize).map(Vec::as_slice)
    }
}

struct Resource(Option<Image>);
impl ImageResource for Resource {
    fn image_info(&self) -> Option<&dyn ImageInfo> {
        self.0.as_ref().map(|image| image as &dyn ImageInfo)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Event {
    Acquire(Service, Option<u64>),
    Retain(Service, u64),
    Release(Service, u64),
    Subscribe(u64, u32),
    Unsubscribe(u64, u32),
    Load(ResourceKey),
    ResourceRelease,
    Create(u32, BindingParams, u32),
    Delete(u32),
    DeleteBatch(Vec<u32>),
    Upload(u32, u32, u32, u32, u32, u32, Vec<u8>),
}

#[derive(Default)]
struct Host {
    events: Vec<Event>,
    image: Option<Image>,
    no_resource: bool,
    missing: Option<Service>,
    next_texture: u32,
    fail_texture: Option<u32>,
    references: u32,
    error_queries: usize,
    upload_error: bool,
    zero_handle: Option<u32>,
    base_destructions: std::rc::Rc<std::cell::Cell<usize>>,
}

impl SystemService for Host {
    fn query_interface(&mut self, iid: u32) -> Option<u32> {
        if iid == 0x287F_B697 {
            self.add_ref();
            Some(0)
        } else {
            None
        }
    }
    fn add_ref(&mut self) -> u32 {
        self.references += 1;
        self.references
    }
    fn release(&mut self) -> u32 {
        self.references -= 1;
        self.references
    }
    fn destroy_base(&mut self) {
        self.base_destructions.set(self.base_destructions.get() + 1);
    }
}

impl Provider for Host {
    type Resource = Resource;
    fn acquire(&mut self, service: Service, graphic: Option<u64>) -> Option<u64> {
        self.events.push(Event::Acquire(service, graphic));
        if self.missing == Some(service) {
            None
        } else {
            Some(match service {
                Service::ResourceManager => 10,
                Service::GraphicSystem => 20,
                Service::Driver => 30,
                Service::MessageServer => 40,
            })
        }
    }
    fn retain(&mut self, service: Service, id: u64) {
        self.events.push(Event::Retain(service, id));
    }
    fn release_service(&mut self, service: Service, id: u64) {
        self.events.push(Event::Release(service, id));
    }
    fn subscribe(&mut self, server: u64, message: u32) {
        self.events.push(Event::Subscribe(server, message));
    }
    fn unsubscribe(&mut self, server: u64, message: u32) {
        self.events.push(Event::Unsubscribe(server, message));
    }
    fn load(&mut self, _manager: u64, key: ResourceKey) -> Option<Resource> {
        self.events.push(Event::Load(key));
        (!self.no_resource).then(|| Resource(self.image.clone()))
    }
    fn release_resource(&mut self, _resource: Resource) {
        self.events.push(Event::ResourceRelease);
    }
    fn create_texture(&mut self, _driver: u64, params: BindingParams, usage: u32) -> u32 {
        self.next_texture += 1;
        self.events.push(Event::Create(self.next_texture, params, usage));
        if self.zero_handle == Some(self.next_texture) {
            0
        } else {
            self.next_texture
        }
    }
    fn driver_error(&mut self, _driver: u64) -> bool {
        self.error_queries += 1;
        self.fail_texture == Some(self.next_texture)
    }
    fn delete_textures(&mut self, _driver: u64, textures: &[u32]) {
        self.events.push(Event::DeleteBatch(textures.to_vec()));
        self.events.extend(textures.iter().copied().map(Event::Delete));
    }
    fn upload(&mut self, _driver: u64, u: Upload<'_>) -> bool {
        self.events.push(Event::Upload(
            u.texture,
            u.level,
            u.width,
            u.height,
            u.format,
            u.component_type,
            u.data.unwrap_or(&[]).to_vec(),
        ));
        !self.upload_error
    }
}

fn factory(image: Image) -> BindingFactory<Host> {
    let mut factory = BindingFactory::new(Host {
        image: Some(image),
        references: 1,
        ..Host::default()
    });
    assert!(factory.init());
    factory.provider_mut().events.clear();
    factory
}

fn key(instance: u32) -> ResourceKey {
    ResourceKey {
        type_id: IMAGE_TYPE,
        group: 100,
        instance,
    }
}

fn events(factory: &BindingFactory<Host>, predicate: fn(&Event) -> bool) -> usize {
    factory
        .provider()
        .events
        .iter()
        .filter(|event| predicate(event))
        .count()
}

#[test]
fn published_sizes_lod_clamp_and_bytes() {
    let p = BindingParams::for_image(Image::new(100, 60, 6, 1, 9).description, 1, 32).unwrap();
    assert_eq!((p.width, p.height, p.levels), (64, 32, 5));
    let p = BindingParams::for_image(Image::new(8, 8, 1, 1, 9).description, 2, 32).unwrap();
    assert_eq!((p.width, p.height, p.levels), (8, 8, 1));
    let p = BindingParams::for_image(Image::new(64, 64, 3, 1, 9).description, 0, 32).unwrap();
    assert_eq!(p.bytes, 5376);
    let p = BindingParams::for_image(Image::new(3, 5, 4, 2, 3).description, 0, 32).unwrap();
    assert_eq!((p.width, p.height, p.bytes), (4, 8, 336)); // (32+8+2+0)*32*2/8
}

#[test]
fn every_format_branch_and_depth_threshold() {
    // Independent expectations at the 23/24-bit split; DXT ignores display and alpha.
    for (format, alpha, depth, internal, bits) in [
        (7, 0, 16, 5, 4),
        (8, 8, 32, 5, 4),
        (9, 1, 16, 6, 8),
        (10, 0, 24, 7, 8),
        (2, 0, 24, 1, 24),
        (3, 8, 24, 4, 32),
        (2, 0, 23, 0, 16),
        (3, 1, 23, 3, 16),
        (3, 2, 23, 2, 16),
    ] {
        let mut d = Image::new(4, 4, 1, 1, format).description;
        d.bits.alpha = alpha;
        let p = BindingParams::for_image(d, 0, depth).unwrap();
        assert_eq!((p.internal_format, p.bits_per_texel), (internal, bits));
    }
}

#[test]
fn dimensions_zero_one_huge_lod_and_no_clamp_byte_accounting() {
    let p = BindingParams::for_image(Image::new(0, 1, 8, 1, 9).description, 0, 32).unwrap();
    assert_eq!((p.width, p.height, p.levels, p.bytes), (1, 1, 8, 1));
    let p = BindingParams::for_image(Image::new(1, 1, 40, 1, 7).description, u32::MAX, 32).unwrap();
    assert_eq!((p.width, p.height, p.levels, p.bytes), (1, 1, 1, 0));
    let p = BindingParams::for_image(Image::new(1, 1, 1, 2, 7).description, 0, 32).unwrap();
    assert_eq!(p.bytes, 1); // divide after multiplying frames, not per frame
}

#[test]
fn signed_power_masked_lod_and_wrapped_sizes() {
    for (word, power) in [
        (0, 1),
        (1, 1),
        (3, 4),
        (0x4000_0000, 0x4000_0000),
        (0x8000_0000, 1),
        (u32::MAX, 1),
    ] {
        assert_eq!(next_power_of_two(word), Ok(power));
    }
    for word in [0x4000_0001, 0x7fff_ffff] {
        assert_eq!(next_power_of_two(word), Err(LayoutError::NativeNontermination));
    }
    for (levels, lod, expected) in [
        (0, 0, (1, 1, 1, 4)),
        (33, 32, (8, 8, 1, 256)),
        (34, 32, (8, 8, 2, 320)),
        (33, u32::MAX, (8, 8, 1, 256)),
    ] {
        let p = BindingParams::for_image(Image::new(8, 8, levels, 1, 3).description, lod, 32).unwrap();
        assert_eq!((p.width, p.height, p.levels, p.bytes), expected);
    }
    for (w, h, frames, bytes) in [
        (65536, 65536, 1, 0),
        (8192, 8192, 2, 0),
        (8, 8, 0, 0),
        (u32::MAX, 1, 1, 4),
    ] {
        let p = BindingParams::for_image(Image::new(w, h, 1, frames, 3).description, 0, 32).unwrap();
        assert_eq!(p.bytes, bytes);
    }
}

#[test]
fn binding_lifetime_pool_transition_and_standalone_deletion() {
    let mut binding = Binding::new(vec![41, 42], true);
    assert_eq!(binding.frame_count(), 2);
    assert_eq!(binding.add_ref(), Some(2));
    assert_eq!(binding.add_ref(), Some(3));
    assert_eq!(binding.release(), Some(Release::Live(2)));
    assert_eq!(binding.release(), Some(Release::FreeToPool));
    assert_eq!(binding.textures(), &[41, 42]);
    let mut standalone = Binding::new(vec![51], false);
    assert_eq!(standalone.release(), Some(Release::Destroyed));
    assert!(standalone.textures().is_empty());
    // Arithmetic-only state: native would have freed the allocation at zero.
    assert_eq!(standalone.release(), Some(Release::Live(u32::MAX)));
    assert_eq!(standalone.add_ref(), Some(0));
}

#[test]
fn key_cache_ignores_type_and_does_not_reload_or_upload_on_reactivation() {
    let mut f = factory(Image::new(64, 64, 1, 1, 7));
    let a = f.get_binding_by_key(key(1)).unwrap();
    let b = f.get_binding_by_key(ResourceKey { type_id: 999, ..key(1) }).unwrap();
    assert_eq!(a, b);
    assert_eq!(f.binding(a).unwrap().reference_count(), 3);
    assert_eq!(f.release_binding(a), Some(2));
    assert_eq!(f.release_binding(b), Some(1));
    assert_eq!(
        f.stats(),
        Stats {
            pooled: 1,
            pooled_bytes: 2048,
            ..Stats::default()
        }
    );
    assert_eq!(f.get_binding_by_key(key(1)), Some(a));
    assert_eq!(events(&f, |e| matches!(e, Event::Load(_))), 1);
    assert_eq!(events(&f, |e| matches!(e, Event::Upload(..))), 1);
    assert_eq!(events(&f, |e| matches!(e, Event::ResourceRelease)), 1);
    assert_eq!(f.stats().in_use_bytes, 2048);
}

#[test]
fn pool_oldest_eviction_and_exact_limit() {
    // Two nonmatching shapes, each 4096 bytes: exact limit keeps both;
    // one byte less must delete the one released first, not the newest.
    let image = Image::new(64, 64, 1, 1, 9);
    let mut f = factory(image.clone());
    f.set_free_pool_max(8192);
    let a = f.get_binding_by_key(key(1)).unwrap();
    let b = f.get_binding_from_image(&Image::new(128, 32, 1, 1, 9)).unwrap();
    f.release_binding(a);
    f.release_binding(b);
    assert_eq!(f.stats().pooled, 2);
    f.set_free_pool_max(8191);
    assert!(f.binding(a).is_none());
    assert!(f.binding(b).is_some());
    assert_eq!(f.stats().pooled_bytes, 4096);
    assert!(f.provider().events.contains(&Event::Delete(1)));
    f.provider_mut().events.clear();
    assert!(f.get_binding_by_key(key(1)).is_some());
    assert_eq!(events(&f, |e| matches!(e, Event::Load(_))), 1);
}

#[test]
fn reuse_oldest_matching_record_unkeys_and_refreshes_bit_counts() {
    let image = Image::new(64, 64, 1, 1, 7);
    let mut f = factory(image.clone());
    let oldest = f.get_binding_by_key(key(1)).unwrap();
    let newer = f.get_binding_by_key(key(2)).unwrap();
    f.release_binding(oldest);
    f.release_binding(newer);
    f.provider_mut().events.clear();
    let mut replacement = image;
    replacement.description.bits = BitCounts {
        red: 5,
        green: 6,
        blue: 5,
        alpha: 0,
    };
    assert_eq!(f.get_binding_from_image(&replacement), Some(oldest));
    assert_eq!(events(&f, |e| matches!(e, Event::Create(..))), 0);
    let mut info = BindingInfo::default();
    f.get_binding_info(oldest, &mut info);
    assert_eq!(info.instance, 0);
    assert_eq!(info.bits, replacement.description.bits);
    assert_eq!(f.get_binding_by_key(key(2)), Some(newer));
    f.get_binding_by_key(key(1)).unwrap();
    assert_eq!(events(&f, |e| matches!(e, Event::Load(_))), 1);
}

#[test]
fn reuse_requires_each_of_the_five_published_fields() {
    let original = Image::new(8, 8, 2, 1, 9);
    for replacement in [
        Image::new(8, 8, 2, 2, 9),
        Image::new(8, 8, 1, 1, 9),
        Image::new(16, 8, 2, 1, 9),
        Image::new(8, 16, 2, 1, 9),
        Image::new(8, 8, 2, 1, 10),
    ] {
        let mut f = factory(original.clone());
        let old = f.get_binding_from_image(&original).unwrap();
        f.release_binding(old);
        assert_ne!(f.get_binding_from_image(&replacement), Some(old));
    }
}

#[test]
fn uploads_skip_largest_levels_in_each_frame() {
    let image = Image::new(100, 60, 6, 2, 9);
    let mut f = factory(image.clone());
    f.set_lod_offset(1);
    f.get_binding_from_image(&image).unwrap();
    let uploads: Vec<_> = f
        .provider()
        .events
        .iter()
        .filter(|e| matches!(e, Event::Upload(..)))
        .cloned()
        .collect();
    assert_eq!(
        uploads,
        vec![
            Event::Upload(1, 0, 64, 32, 9, 1, vec![1]),
            Event::Upload(1, 1, 32, 16, 9, 1, vec![2]),
            Event::Upload(1, 2, 16, 8, 9, 1, vec![3]),
            Event::Upload(1, 3, 8, 4, 9, 1, vec![4]),
            Event::Upload(1, 4, 4, 2, 9, 1, vec![5]),
            Event::Upload(2, 0, 64, 32, 9, 1, vec![17]),
            Event::Upload(2, 1, 32, 16, 9, 1, vec![18]),
            Event::Upload(2, 2, 16, 8, 9, 1, vec![19]),
            Event::Upload(2, 3, 8, 4, 9, 1, vec![20]),
            Event::Upload(2, 4, 4, 2, 9, 1, vec![21]),
        ]
    );
}

#[test]
fn partial_driver_failure_excludes_current_handle_and_releases_resource() {
    let mut f = factory(Image::new(8, 8, 1, 3, 9));
    f.provider_mut().fail_texture = Some(2);
    assert!(f.get_binding_by_key(key(1)).is_none());
    assert_eq!(f.stats(), Stats::default());
    assert_eq!(events(&f, |e| matches!(e, Event::Create(..))), 2);
    assert!(f
        .provider()
        .events
        .ends_with(&[Event::DeleteBatch(vec![1]), Event::Delete(1), Event::ResourceRelease]));
    assert_eq!(events(&f, |e| matches!(e, Event::Upload(..))), 0);
    f.provider_mut().fail_texture = None;
    assert!(f.get_binding_by_key(key(1)).is_some());
    assert_eq!(events(&f, |e| matches!(e, Event::Load(_))), 2);
}

#[test]
fn missing_resource_and_null_image_fail_without_records() {
    let mut f = factory(Image::new(8, 8, 1, 1, 9));
    f.provider_mut().no_resource = true;
    assert!(f.get_binding_by_key(key(1)).is_none());
    assert_eq!(events(&f, |e| matches!(e, Event::ResourceRelease)), 0);
    f.provider_mut().no_resource = false;
    f.provider_mut().image = None;
    assert!(f.get_binding_by_key(key(1)).is_none());
    assert_eq!(events(&f, |e| matches!(e, Event::ResourceRelease)), 1);
    assert_eq!(f.stats(), Stats::default());
}

#[test]
fn restore_only_keyed_records_including_pool_and_forces_fsh_type() {
    let image = Image::new(8, 8, 1, 1, 9);
    let mut f = factory(image.clone());
    let pooled = f.get_binding_by_key(ResourceKey { type_id: 123, ..key(1) }).unwrap();
    f.get_binding_from_image(&image).unwrap();
    f.release_binding(pooled);
    let before = f.stats();
    f.provider_mut().events.clear();
    assert!(f.do_message(0));
    assert!(f.provider().events.is_empty());
    assert!(f.do_message(RESTORE_TEXTURES));
    assert_eq!(f.stats(), before);
    assert_eq!(f.binding(pooled).unwrap().reference_count(), 1);
    let loads: Vec<_> = f
        .provider()
        .events
        .iter()
        .filter(|e| matches!(e, Event::Load(_)))
        .cloned()
        .collect();
    assert_eq!(loads, vec![Event::Load(key(1))]);
    assert_eq!(events(&f, |e| matches!(e, Event::Upload(..))), 1);
    assert_eq!(events(&f, |e| matches!(e, Event::ResourceRelease)), 1);
}

#[test]
fn resource_group_change_unkeys_without_destroying_and_id_lookup_is_hardcoded() {
    let mut f = factory(Image::new(8, 8, 1, 1, 9));
    let a = f.get_binding_by_key(key(1)).unwrap();
    f.set_resource_group(DEFAULT_GROUP);
    assert_eq!(f.get_binding_by_key(key(1)), Some(a)); // unchanged setting preserves cache
    f.set_resource_group(777);
    let mut info = BindingInfo::default();
    f.get_binding_info(a, &mut info);
    assert_eq!((info.group, info.instance), (100, 0));
    assert!(f.binding(a).is_some());
    f.provider_mut().events.clear();
    f.do_message(RESTORE_TEXTURES);
    assert!(f.provider().events.is_empty());
    f.get_binding_by_id(99).unwrap();
    assert!(f.provider().events.contains(&Event::Load(ResourceKey {
        type_id: IMAGE_TYPE,
        group: DEFAULT_GROUP,
        instance: 99
    })));
    assert_eq!(f.resource_group(), 777);
}

#[test]
fn flush_pool_restores_limit_flush_all_invalidates_live_handles_and_unknown_info_is_untouched() {
    let image = Image::new(8, 8, 1, 1, 9);
    let mut f = factory(image.clone());
    let a = f.get_binding_from_image(&image).unwrap();
    let b = f.get_binding_from_image(&image).unwrap();
    f.release_binding(a);
    f.flush_free_pool();
    assert!(f.binding(a).is_none());
    assert!(f.binding(b).is_some());
    assert_eq!(f.free_pool_max(), 2_000_000);
    let mut info = BindingInfo {
        instance: 777,
        ..BindingInfo::default()
    };
    let before = info;
    f.get_binding_info(a, &mut info);
    assert_eq!(info, before);
    f.set_binding_image_data(a, &image);
    f.flush_all();
    assert!(f.binding(b).is_none());
    assert_eq!(f.release_binding(b), None);
    assert_eq!(f.stats(), Stats::default());
    let c = f.get_binding_from_image(&image).unwrap();
    assert_ne!(c, a);
    assert_ne!(c, b);
}

#[test]
fn zero_byte_pool_record_survives_zero_limit_but_not_flush_all() {
    let image = Image::new(1, 1, 1, 1, 7);
    let mut f = factory(image.clone());
    f.set_free_pool_max(0);
    let a = f.get_binding_from_image(&image).unwrap();
    f.release_binding(a);
    f.flush_free_pool();
    assert_eq!(f.stats().pooled, 1); // literal while bytes > limit, not while count > 0
    f.flush_all();
    assert!(f.binding(a).is_none());
}

#[test]
fn immediate_pool_eviction_still_returns_one() {
    let image = Image::new(8, 8, 1, 1, 9);
    let mut f = factory(image.clone());
    f.set_free_pool_max(0);
    let a = f.get_binding_from_image(&image).unwrap();
    assert_eq!(f.release_binding(a), Some(1));
    assert!(f.binding(a).is_none());
}

#[test]
fn defaults_stats_and_display_lod_changes_affect_only_new_records() {
    let image = Image::new(8, 8, 3, 1, 3);
    let mut f = factory(image.clone());
    assert_eq!(
        (f.lod_offset(), f.display_depth(), f.resource_group(), f.free_pool_max()),
        (0, 32, DEFAULT_GROUP, 2_000_000)
    );
    let a = f.get_binding_by_key(key(1)).unwrap();
    f.set_lod_offset(2);
    f.set_display_depth(16);
    assert_eq!(f.get_binding_by_key(key(1)), Some(a));
    let b = f.get_binding_from_image(&image).unwrap();
    let mut info = BindingInfo::default();
    f.get_binding_info(a, &mut info);
    assert_eq!((info.width, info.levels, info.bits_per_texel), (8, 3, 32));
    f.get_binding_info(b, &mut info);
    assert_eq!((info.width, info.levels, info.bits_per_texel), (2, 1, 16));
    assert_eq!(
        Stats {
            in_use: 2,
            in_use_bytes: 2 << 20,
            pooled: 1,
            pooled_bytes: 1 << 19
        }
        .format(),
        "Textures: 2 (2.0 MB) in use, 1 (0.5 MB) awaiting reuse, 3 (2.5 MB) total\n"
    );
    assert_eq!(f.stats_string(), f.stats().format());
}

#[test]
fn service_init_failure_order_optional_messages_and_shutdown_reference_balance() {
    for missing in [
        Service::ResourceManager,
        Service::GraphicSystem,
        Service::Driver,
        Service::MessageServer,
    ] {
        let mut f = BindingFactory::new(Host {
            missing: Some(missing),
            ..Host::default()
        });
        assert_eq!(f.init(), missing == Service::MessageServer);
        f.shutdown();
        let acquired: Vec<_> = f
            .provider()
            .events
            .iter()
            .filter_map(|e| match e {
                Event::Acquire(service, _) if *service != missing => Some(*service),
                _ => None,
            })
            .collect();
        let released: Vec<_> = f
            .provider()
            .events
            .iter()
            .filter_map(|e| match e {
                Event::Release(service, _) => Some(*service),
                _ => None,
            })
            .collect();
        assert_eq!(acquired.len(), released.len());
        for service in acquired {
            assert!(released.contains(&service));
        }
    }
    let mut f = factory(Image::new(8, 8, 1, 1, 9));
    f.get_binding_by_key(key(1)).unwrap();
    f.provider_mut().events.clear();
    f.shutdown();
    assert_eq!(
        f.provider().events,
        vec![
            Event::Delete(1),
            Event::Release(Service::Driver, 30),
            Event::Release(Service::GraphicSystem, 20),
            Event::Release(Service::ResourceManager, 10),
            Event::Unsubscribe(40, RESTORE_TEXTURES),
            Event::Release(Service::MessageServer, 40)
        ]
    );
    f.provider_mut().events.clear();
    f.shutdown();
    assert!(f.provider().events.is_empty());
}

#[test]
fn driver_identity_change_flushes_before_release_and_retains_only_new_pointer() {
    let mut f = factory(Image::new(8, 8, 1, 1, 9));
    let live = f.get_binding_by_key(key(1)).unwrap();
    f.provider_mut().events.clear();
    assert!(!f.set_driver(Some(30)));
    assert!(f.provider().events.is_empty());
    assert!(f.set_driver(Some(55)));
    assert_eq!(f.driver(), Some(55));
    assert!(f.binding(live).is_none());
    assert_eq!(
        f.provider().events,
        vec![
            Event::Delete(1),
            Event::Release(Service::Driver, 30),
            Event::Retain(Service::Driver, 55)
        ]
    );
    assert!(f.set_driver(None));
    assert_eq!(f.provider().events.last(), Some(&Event::Release(Service::Driver, 55)));
}

#[test]
fn factory_interfaces_share_base_count_and_release_zero_shuts_down() {
    let mut f = factory(Image::new(8, 8, 1, 1, 9));
    assert_eq!(f.query_interface(1), Some(0));
    assert_eq!(f.query_interface(0xFAC0_E219), Some(0));
    assert_eq!(f.query_interface(0x088C_EB67), Some(4));
    assert_eq!(f.query_interface(0x287F_B697), Some(8));
    assert_eq!(f.query_interface(999), None);
    assert_eq!(f.provider().references, 5);
    assert_eq!(f.add_ref(), Some(6));
    for n in (0..6).rev() {
        assert_eq!(f.release(), Some(n));
    }
    assert_eq!(f.driver(), None);
    assert!(!f.init());
    assert_eq!(f.query_interface(1), None);
}

struct Record {
    flags: u32,
    textures: Vec<MaterialTexture>,
    threshold: u16,
}
impl MaterialRecord for Record {
    fn stored_flags(&self) -> u32 {
        self.flags
    }
    fn alpha_func(&self) -> u8 {
        6
    }
    fn depth_func(&self) -> u8 {
        2
    }
    fn src_blend(&self) -> u8 {
        4
    }
    fn dst_blend(&self) -> u8 {
        5
    }
    fn alpha_threshold(&self) -> u16 {
        self.threshold
    }
    fn texture_count(&self) -> usize {
        self.textures.len()
    }
    fn texture(&self, index: usize) -> MaterialTexture {
        self.textures[index]
    }
}

fn material_record(count: usize) -> Record {
    Record {
        flags: 0x0001_0022,
        threshold: 0x7FFF,
        textures: (0..count)
            .map(|index| MaterialTexture {
                id: 100 + index as u32,
                wrap_s: 2,
                wrap_t: 3,
                mag_filter: 0,
                min_filter: 7,
            })
            .collect(),
    }
}

#[test]
fn material_published_vector_preserves_high_byte_and_slot_defaults() {
    let manager = MaterialManager::new(Host::default());
    let material = manager.create_material(&material_record(1)).unwrap();
    assert_eq!(
        (material.first_byte, material.references, material.flags),
        (1, 1, 0x01A2)
    );
    assert_eq!(
        (
            material.alpha_func,
            material.depth_func,
            material.src_blend,
            material.dst_blend,
            material.alpha_threshold
        ),
        (6, 2, 4, 5, 0x7F)
    );
    assert_eq!(
        material.textures[0],
        TextureSlot {
            id: Some(100),
            wrap_s: 2,
            wrap_t: 3,
            mag_filter: 0,
            min_filter: 7,
            ..TextureSlot::default()
        }
    );
    assert_eq!(
        material.textures[1],
        TextureSlot {
            environment: 0xFF,
            ..TextureSlot::default()
        }
    );
    assert_eq!(manager.stats_string(), "Nothing to report");
    assert!(!manager.create_material_variant());
}

#[test]
fn material_zero_two_and_excess_texture_boundaries_and_thresholds() {
    let manager = MaterialManager::new(Host::default());
    let empty = manager.create_material(&material_record(0)).unwrap();
    assert!(empty
        .textures
        .iter()
        .all(|slot| slot.environment == 0xFF && slot.id.is_none()));
    let two = manager.create_material(&material_record(2)).unwrap();
    assert_eq!(two.textures[1].id, Some(101));
    assert!(two
        .textures
        .iter()
        .all(|slot| slot.binding.is_none() && slot.environment == 1));
    assert!(manager.create_material(&material_record(3)).is_none());
    for (threshold, expected) in [(0x00FF, 0), (0x0100, 1), (0xFF00, 255), (0xFFFF, 255)] {
        let mut record = material_record(0);
        record.threshold = threshold;
        record.flags = 0xFFFF_0000;
        let material = manager.create_material(&record).unwrap();
        assert_eq!(material.alpha_threshold, expected);
        assert_eq!(material.flags, 0x180);
    }
}

#[test]
fn material_manager_interface_fallback_and_deleting_destructor_flags() {
    let mut manager = MaterialManager::new(Host {
        references: 1,
        ..Host::default()
    });
    assert_eq!(manager.query_interface(0xFAC0_E21A), Some(0));
    assert_eq!(manager.query_interface(0x287F_B697), Some(4));
    assert_eq!(manager.query_interface(99), None);
    assert_eq!(manager.release(), Some(2));
    assert_eq!(manager.release(), Some(1));
    assert_eq!(manager.release(), Some(0));
    assert_eq!(manager.query_interface(0xFAC0_E21A), None);
    assert!(manager.create_material(&material_record(0)).is_none());
    for flags in 0..4 {
        let mut manager = MaterialManager::new(Host::default());
        assert_eq!(manager.deleting_destructor(flags), flags & 1 != 0);
    }
}

#[test]
fn reuse_compares_internal_format_not_source_format_or_bit_counts() {
    let rgb = Image::new(8, 8, 1, 1, 7);
    let mut rgba = Image::new(8, 8, 1, 1, 8);
    rgba.description.bits.alpha = 1;
    let mut f = factory(rgb.clone());
    let old = f.get_binding_from_image(&rgb).unwrap();
    f.release_binding(old);
    f.provider_mut().events.clear();
    assert_eq!(f.get_binding_from_image(&rgba), Some(old));
    assert_eq!(events(&f, |e| matches!(e, Event::Create(..))), 0);
    assert!(f.provider().events.contains(&Event::Upload(1, 0, 8, 8, 8, 1, vec![0])));
}

#[test]
fn upload_dimensions_halve_to_zero_and_are_not_clamped() {
    let image = Image::new(1, 4, 4, 1, 9);
    let mut f = factory(image.clone());
    f.get_binding_from_image(&image).unwrap();
    let uploads: Vec<_> = f
        .provider()
        .events
        .iter()
        .filter(|e| matches!(e, Event::Upload(..)))
        .cloned()
        .collect();
    assert_eq!(
        uploads,
        vec![
            Event::Upload(1, 0, 1, 4, 9, 1, vec![0]),
            Event::Upload(1, 1, 0, 2, 9, 1, vec![1]),
            Event::Upload(1, 2, 0, 1, 9, 1, vec![2]),
            Event::Upload(1, 3, 0, 0, 9, 1, vec![3])
        ]
    );
}

#[test]
fn init_shuts_down_existing_records_before_acquiring_services_again() {
    let mut f = factory(Image::new(8, 8, 1, 1, 9));
    let old = f.get_binding_by_key(key(1)).unwrap();
    f.provider_mut().events.clear();
    assert!(f.init());
    assert!(f.binding(old).is_none());
    assert_eq!(
        f.provider().events,
        vec![
            Event::Delete(1),
            Event::Release(Service::Driver, 30),
            Event::Release(Service::GraphicSystem, 20),
            Event::Release(Service::ResourceManager, 10),
            Event::Unsubscribe(40, RESTORE_TEXTURES),
            Event::Release(Service::MessageServer, 40),
            Event::Acquire(Service::ResourceManager, None),
            Event::Acquire(Service::GraphicSystem, None),
            Event::Acquire(Service::Driver, Some(20)),
            Event::Acquire(Service::MessageServer, None),
            Event::Subscribe(40, RESTORE_TEXTURES)
        ]
    );
}

#[test]
fn failed_restore_does_not_change_binding_membership_or_references() {
    let mut f = factory(Image::new(8, 8, 1, 1, 9));
    let old = f.get_binding_by_key(key(1)).unwrap();
    f.provider_mut().events.clear();
    f.provider_mut().no_resource = true;
    assert!(f.do_message(RESTORE_TEXTURES));
    assert_eq!(f.provider().events, vec![Event::Load(key(1))]);
    f.provider_mut().no_resource = false;
    f.provider_mut().image = None;
    f.provider_mut().events.clear();
    assert!(f.do_message(RESTORE_TEXTURES));
    assert_eq!(f.provider().events, vec![Event::Load(key(1)), Event::ResourceRelease]);
    assert_eq!(f.binding(old).unwrap().reference_count(), 2);
    assert_eq!(f.stats().in_use, 1);
}

#[test]
fn no_driver_no_manager_fail_and_zero_key_is_accepted() {
    let image = Image::new(8, 8, 1, 1, 9);
    let mut f = BindingFactory::new(Host::default());
    assert!(f.get_binding_from_image(&image).is_none());
    assert_eq!(f.unsupported(), Some(Unsupported::MissingService));
    assert!(f.get_binding_by_key(key(1)).is_none());
    assert_eq!(f.unsupported(), Some(Unsupported::MissingService));
    assert!(f.provider().events.is_empty());
    let mut f = factory(image);
    assert!(f.get_binding_by_key(key(0)).is_some());
    assert_eq!(events(&f, |e| matches!(e, Event::Load(_))), 1);
}

#[test]
fn factory_deleting_destructor_cleans_records_for_every_flag() {
    for flags in 0..4 {
        let mut f = factory(Image::new(8, 8, 1, 1, 9));
        let old = f.get_binding_by_key(key(1)).unwrap();
        assert_eq!(f.deleting_destructor(flags), flags & 1 != 0);
        assert!(f.binding(old).is_none());
        assert_eq!(f.stats(), Stats::default());
        assert_eq!(f.add_ref(), None);
        assert!(f.provider().events.contains(&Event::Delete(1)));
    }
}

#[test]
fn creation_failure_first_and_third_bulk_counts_and_zero_handles() {
    for (failure, expected) in [(1, vec![]), (3, vec![0, 2])] {
        let mut f = factory(Image::new(8, 8, 1, 3, 9));
        f.provider_mut().fail_texture = Some(failure);
        f.provider_mut().zero_handle = Some(1);
        assert!(f.get_binding_by_key(key(17)).is_none());
        assert_eq!(f.unsupported(), None);
        assert_eq!(f.provider().error_queries, failure as usize);
        let batches: Vec<_> = f
            .provider()
            .events
            .iter()
            .filter_map(|event| {
                if let Event::DeleteBatch(handles) = event {
                    Some(handles.clone())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(batches, vec![expected]);
        assert_eq!(f.provider().events.last(), Some(&Event::ResourceRelease));
        assert_eq!(f.stats(), Stats::default());
    }
}

#[test]
fn zero_frame_creation_and_zero_level_memory_boundary_are_distinct() {
    let mut bare = BindingFactory::new(Host::default());
    assert!(bare.get_binding_from_image(&Image::new(8, 8, 0, 0, 3)).is_some());
    assert!(bare.provider().events.is_empty());
    assert_eq!(bare.unsupported(), None);
    let mut f = factory(Image::new(8, 8, 1, 0, 3));
    let id = f.get_binding_by_key(key(1)).unwrap();
    assert_eq!(f.binding(id).unwrap().frame_count(), 0);
    assert_eq!(f.stats().in_use_bytes, 0);
    assert_eq!(events(&f, |e| matches!(e, Event::Create(..) | Event::Upload(..))), 0);
    assert_eq!(f.unsupported(), None);
    let zero = Image::new(8, 8, 0, 1, 3);
    f.get_binding_from_image(&zero);
    assert_eq!(f.unsupported(), Some(Unsupported::ImageTable));
    assert_eq!(
        events(
            &f,
            |e| matches!(e, Event::Create(_, p, _) if p.levels == 1 && p.width == 1 && p.height == 1)
        ),
        1
    );
    let mut huge = zero;
    huge.description.width = 0x4000_0001;
    assert!(f.get_binding_from_image(&huge).is_none());
    assert_eq!(f.unsupported(), Some(Unsupported::NativeNontermination));
}

#[test]
fn instance_zero_reacquires_then_reuses_stale_key_and_restore_uses_record_key() {
    let image = Image::new(8, 8, 1, 1, 9);
    let mut f = factory(image.clone());
    let a = f.get_binding_by_key(key(0)).unwrap();
    f.release_binding(a);
    f.provider_mut().events.clear();
    assert_eq!(f.get_binding_by_key(key(0)), Some(a));
    assert!(f.provider().events.is_empty());
    f.release_binding(a);
    assert_eq!(f.get_binding_from_image(&image), Some(a));
    f.provider_mut().events.clear();
    assert!(f.do_message(RESTORE_TEXTURES));
    assert_eq!(f.unsupported(), None);
    assert_eq!(f.provider().events.first(), Some(&Event::Load(key(0))));
    assert_eq!(events(&f, |e| matches!(e, Event::Upload(..))), 1);
    assert_eq!(f.get_binding_by_key(key(0)), Some(a));
    f.release_binding(a);
    f.release_binding(a);
    assert_eq!(f.get_binding_by_key(key(7)), Some(a));
    f.provider_mut().events.clear();
    assert_eq!(f.get_binding_by_key(key(0)), Some(a));
    assert!(f.provider().events.is_empty());
    let mut info = BindingInfo::default();
    f.get_binding_info(a, &mut info);
    assert_eq!((info.group, info.instance), (100, 7));
    assert!(f.do_message(RESTORE_TEXTURES));
    assert_eq!(f.unsupported(), None);
    let loads: Vec<_> = f
        .provider()
        .events
        .iter()
        .filter(|e| matches!(e, Event::Load(_)))
        .cloned()
        .collect();
    assert_eq!(loads, [Event::Load(key(7)), Event::Load(key(7))]);
    assert_eq!(events(&f, |e| matches!(e, Event::Upload(..))), 2);
}

#[test]
fn instance_zero_eviction_reports_dangling_memory_instead_of_native_false() {
    let mut f = factory(Image::new(8, 8, 1, 1, 9));
    let id = f.get_binding_by_key(key(0)).unwrap();
    f.set_free_pool_max(0);
    f.release_binding(id);
    assert!(f.get_binding_by_key(key(0)).is_none());
    assert_eq!(f.unsupported(), Some(Unsupported::DanglingKey));
    assert!(f.do_message(RESTORE_TEXTURES));
    assert_eq!(f.unsupported(), Some(Unsupported::DanglingKey));
    f.flush_all();
    assert!(f.get_binding_by_key(key(0)).is_some());
    assert_eq!(f.unsupported(), None);
}

#[test]
fn source_frame_count_ignored_upload_errors_and_masked_upload_shifts() {
    let image = Image::new(8, 8, 34, 2, 9);
    let mut f = factory(image.clone());
    f.provider_mut().upload_error = true;
    let id = f.get_binding_from_image(&image).unwrap();
    assert_eq!(f.provider().error_queries, 2); // creation only
    assert_eq!(events(&f, |e| matches!(e, Event::Upload(..))), 68);
    let uploads: Vec<_> = f
        .provider()
        .events
        .iter()
        .filter_map(|e| {
            if let Event::Upload(t, l, w, h, _, _, data) = e {
                Some((*t, *l, *w, *h, data.clone()))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(uploads[31], (1, 31, 0, 0, vec![31]));
    assert_eq!(uploads[32], (1, 32, 8, 8, vec![32]));
    assert_eq!(uploads[34], (2, 0, 8, 8, vec![16]));
    f.provider_mut().events.clear();
    f.set_binding_image_data(id, &Image::new(8, 8, 34, 1, 9));
    assert_eq!(events(&f, |e| matches!(e, Event::Upload(..))), 34);
    assert_eq!(f.unsupported(), None);
    f.provider_mut().events.clear();
    f.set_binding_image_data(id, &Image::new(8, 8, 34, 3, 9));
    assert_eq!(events(&f, |e| matches!(e, Event::Upload(..))), 68);
    assert_eq!(f.unsupported(), Some(Unsupported::BindingHandleArray));
    f.provider_mut().events.clear();
    f.set_binding_image_data(id, &Image::new(8, 8, 1, 1, 9));
    assert_eq!(f.unsupported(), Some(Unsupported::ImageTable));
    assert!(f.provider().events.is_empty());
}

#[test]
fn null_pointer_is_forwarded_and_does_not_stop_later_levels() {
    struct NullImage(Image);
    impl ImageInfo for NullImage {
        fn description(&self) -> ImageDescription {
            self.0.description
        }
        fn level_data(&self, _frame: u32, _level: u32) -> Option<&[u8]> {
            None
        }
        fn level_pointer(&self, _frame: u32, _level: u32) -> Result<Option<&[u8]>, Unsupported> {
            Ok(None)
        }
    }
    let image = NullImage(Image::new(8, 8, 2, 2, 9));
    let mut f = factory(image.0.clone());
    f.get_binding_from_image(&image).unwrap();
    assert_eq!(
        events(
            &f,
            |e| matches!(e, Event::Upload(_, _, _, _, _, _, data) if data.is_empty())
        ),
        4
    );
    assert_eq!(f.unsupported(), None);
}

#[test]
fn accounting_wraps_on_create_pool_reacquire_and_reuse() {
    let image = Image::new(8192, 8192, 1, 1, 3);
    let mut f = factory(image.clone());
    f.set_free_pool_max(u32::MAX);
    let ids: Vec<_> = (1..=17).map(|i| f.get_binding_by_key(key(i)).unwrap()).collect();
    assert_eq!(
        f.stats(),
        Stats {
            in_use: 17,
            in_use_bytes: 0x1000_0000,
            pooled: 0,
            pooled_bytes: 0
        }
    );
    for id in &ids {
        f.release_binding(*id);
    }
    assert_eq!(
        f.stats(),
        Stats {
            in_use: 0,
            in_use_bytes: 0,
            pooled: 17,
            pooled_bytes: 0x1000_0000
        }
    );
    // Wrapped pool bytes are below the limit despite the large mathematical sum.
    f.set_free_pool_max(0x1000_0000);
    assert_eq!(f.stats().pooled, 17);
    assert_eq!(f.get_binding_by_key(key(1)), Some(ids[0]));
    assert_eq!(f.stats().pooled_bytes, 0);
    f.get_binding_from_image(&image).unwrap();
    assert_eq!(f.stats().pooled_bytes, 0xf000_0000);
    assert_eq!(f.stats().in_use_bytes, 0x2000_0000);
    f.set_free_pool_max(0);
    assert_eq!(f.stats().pooled_bytes, 0);
    assert_eq!(f.stats().pooled, 0);
}

#[derive(Default)]
struct BindingCalls(Vec<(bool, BindingId)>);
impl BindingReferences for BindingCalls {
    fn acquire_binding(&mut self, id: BindingId) {
        self.0.push((true, id));
    }
    fn release_binding(&mut self, id: BindingId) {
        self.0.push((false, id));
    }
}

fn material_with_bindings(manager: &MaterialManager<Host>, a: BindingId, b: BindingId) -> Material {
    let mut m = manager.create_material(&material_record(2)).unwrap();
    m.textures[0].binding = Some(a);
    m.textures[1].binding = Some(b);
    m
}

#[test]
fn material_counts_zero_one_two_65535_and_reverse_stage_release() {
    let mut f = factory(Image::new(8, 8, 1, 1, 9));
    let a = f.get_binding_by_key(key(1)).unwrap();
    let b = f.get_binding_by_key(key(2)).unwrap();
    let manager = MaterialManager::new(Host::default());
    for count in [0, 1, 2, 65535] {
        let mut m = material_with_bindings(&manager, a, b);
        m.references = count;
        m.textures[1].environment = 0xff; // unused markers do not inhibit release
        let mut arena = MaterialArena::default();
        let id = arena.insert(m);
        let mut calls = BindingCalls::default();
        assert_eq!(
            arena.release(id, &mut calls),
            Ok(if count > 1 { u32::from(count - 1) } else { 0 })
        );
        if count > 1 {
            assert_eq!(arena.get(id).unwrap().references, count - 1);
            assert!(calls.0.is_empty());
        } else {
            assert!(arena.get(id).is_none());
            assert_eq!(calls.0, [(false, b), (false, a)]);
        }
    }
}

#[test]
fn material_reference_assignment_wraps_new_count_and_handles_equal_and_null() {
    let mut f = factory(Image::new(8, 8, 1, 1, 9));
    let a = f.get_binding_by_key(key(1)).unwrap();
    let b = f.get_binding_by_key(key(2)).unwrap();
    let manager = MaterialManager::new(Host::default());
    let mut arena = MaterialArena::default();
    let old = arena.insert(material_with_bindings(&manager, a, b));
    let mut m = manager.create_material(&material_record(0)).unwrap();
    m.references = 65535;
    let new = arena.insert(m);
    let mut target = Some(old);
    let mut calls = BindingCalls::default();
    arena.assign_reference(&mut target, Some(new), &mut calls).unwrap();
    assert_eq!(target, Some(new));
    assert_eq!(arena.get(new).unwrap().references, 0);
    assert!(arena.get(old).is_none());
    assert_eq!(calls.0, [(false, b), (false, a)]);
    calls.0.clear();
    arena.assign_reference(&mut target, Some(new), &mut calls).unwrap();
    assert_eq!(arena.get(new).unwrap().references, 0);
    assert!(calls.0.is_empty());
    arena.assign_reference(&mut target, None, &mut calls).unwrap();
    assert_eq!(target, None);
    assert!(arena.get(new).is_none());
}

#[test]
fn material_value_assignment_copies_count_and_acquires_before_release_per_stage() {
    let mut f = factory(Image::new(8, 8, 1, 1, 9));
    let a = f.get_binding_by_key(key(1)).unwrap();
    let b = f.get_binding_by_key(key(2)).unwrap();
    let c = f.get_binding_by_key(key(3)).unwrap();
    let manager = MaterialManager::new(Host::default());
    let mut target = material_with_bindings(&manager, a, b);
    let mut source = material_with_bindings(&manager, b, c);
    source.references = 65535;
    source.first_byte = 7;
    source.flags = 0x3456;
    source.header_tail = Some(0xa5);
    source.textures[0].scale_divisor = 3.5;
    let mut calls = BindingCalls::default();
    target.assign_value(&source, &mut calls);
    assert_eq!(target, source);
    assert_eq!(calls.0, [(true, b), (false, a), (true, c), (false, b)]);
    calls.0.clear();
    target.assign_value(&source, &mut calls);
    assert!(calls.0.is_empty());
    source.textures[0].binding = None;
    source.textures[1].binding = Some(a);
    target.assign_value(&source, &mut calls);
    assert_eq!(calls.0, [(false, b), (true, a), (false, c)]);
}

#[test]
fn value_range_cleanup_is_forward_ignores_counts_and_preserves_enclosing_values() {
    let mut f = factory(Image::new(8, 8, 1, 1, 9));
    let a = f.get_binding_by_key(key(1)).unwrap();
    let b = f.get_binding_by_key(key(2)).unwrap();
    let c = f.get_binding_by_key(key(3)).unwrap();
    let manager = MaterialManager::new(Host::default());
    let mut values = [
        material_with_bindings(&manager, a, b),
        material_with_bindings(&manager, c, a),
    ];
    values[0].references = 65535;
    values[1].references = 0;
    let mut calls = BindingCalls::default();
    release_value_range(&values, &mut calls);
    assert_eq!(calls.0, [(false, b), (false, a), (false, a), (false, c)]);
    assert_eq!((values[0].references, values[1].references), (65535, 0));
}

#[test]
fn manager_false_slots_and_base_destruction_do_not_enumerate_materials() {
    let host = Host::default();
    let destructions = host.base_destructions.clone();
    let mut manager = MaterialManager::new(host);
    assert!(!manager.slot_6(0, u32::MAX));
    assert!(!manager.slot_7(u32::MAX, 0));
    let m = manager.create_material(&material_record(1)).unwrap();
    assert!(!manager.deleting_destructor(2));
    assert_eq!(destructions.get(), 1);
    drop(manager);
    assert_eq!(destructions.get(), 1);
    assert_eq!(m.references, 1);
}

#[test]
fn constructor_count_and_material_binding_lifetime_reach_the_real_factory() {
    let raw = Material::constructor();
    assert_eq!(raw.references, 0);
    assert_eq!(raw.header_tail, None);
    assert!(raw.textures.iter().all(|slot| *slot == TextureSlot::default()));
    let mut f = factory(Image::new(8, 8, 1, 1, 9));
    let a = f.get_binding_by_key(key(1)).unwrap();
    let b = f.get_binding_by_key(key(2)).unwrap();
    let manager = MaterialManager::new(Host::default());
    assert_eq!(
        manager.try_create_material(&material_record(3)),
        Err(Unsupported::MaterialSlots)
    );
    let mut arena = MaterialArena::default();
    let id = arena.insert(material_with_bindings(&manager, a, b));
    assert_eq!(arena.release(id, &mut f), Ok(0));
    assert_eq!((f.stats().in_use, f.stats().pooled), (0, 2));
    assert_eq!(f.binding(a).unwrap().reference_count(), 1);
    assert_eq!(f.binding(b).unwrap().reference_count(), 1);
}

#[test]
fn restore_distinct_records_withholds_unknown_hash_order() {
    let mut f = factory(Image::new(8, 8, 1, 1, 9));
    f.get_binding_by_key(key(9)).unwrap();
    f.get_binding_by_key(key(1)).unwrap();
    f.provider_mut().events.clear();
    assert!(f.do_message(RESTORE_TEXTURES));
    assert_eq!(f.unsupported(), Some(Unsupported::RestoreTableOrder));
    assert!(f.provider().events.is_empty());
}

#[test]
fn wrapped_pool_total_can_stop_trimming_with_sixteen_records_remaining() {
    let mut f = factory(Image::new(8192, 8192, 1, 1, 3));
    f.set_free_pool_max(u32::MAX);
    for i in 1..=17 {
        let id = f.get_binding_by_key(key(i)).unwrap();
        // Keep every record live until all have been created, preventing reuse.
        assert_eq!(f.binding(id).unwrap().reference_count(), 2);
    }
    for i in 1..=17 {
        let id = f.get_binding_by_key(key(i)).unwrap();
        f.release_binding(id);
        f.release_binding(id);
    }
    assert_eq!(f.stats().pooled_bytes, 0x1000_0000);
    f.set_free_pool_max(0);
    assert_eq!((f.stats().pooled, f.stats().pooled_bytes), (16, 0));
}
