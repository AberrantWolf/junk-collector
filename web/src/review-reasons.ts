import type {components} from './generated/api';
export type ReviewReason=components['schemas']['MetadataReviewReason'];
export type ReviewLevel=components['schemas']['ReviewLevel'];

// Plain-language labels for list rows and filters. The server decides which
// reasons apply and how urgent each is; this only names them.
export const reasonLabels:Record<ReviewReason,string>={
 missing_name:'No name yet',
 missing_artwork:'No cover',
 album_ambiguity:'Several possible releases',
 incomplete_provider_information:'Incomplete online info',
 no_match:'No match found',
 lookup_failed:'Lookup failed',
 manual_conflict:'Your name differs from the match',
 grouping_proposal:'Possibly one album with another entry',
 catalog_ambiguity:'Several catalog entries match',
 weak_catalog_match:'Only a partial catalog match',
 unsupported_format:'Format not supported',
 needs_main_file:'Choose the main file',
 no_catalog:'Waiting for a game catalog',
};

// What the fix panel offers first for each reason.
export const reasonActions:Record<ReviewReason,string>={
 missing_name:'Name it',
 missing_artwork:'Add a cover',
 album_ambiguity:'Choose',
 incomplete_provider_information:'Review',
 no_match:'Search',
 lookup_failed:'Retry',
 manual_conflict:'Compare',
 grouping_proposal:'Compare',
 catalog_ambiguity:'Choose',
 weak_catalog_match:'Review',
 unsupported_format:'Name it',
 needs_main_file:'Choose',
 no_catalog:'Game catalogs',
};

export const reasonLevels:Record<ReviewReason,ReviewLevel>={
 missing_name:'decision',missing_artwork:'polish',album_ambiguity:'decision',incomplete_provider_information:'polish',
 no_match:'decision',lookup_failed:'decision',manual_conflict:'decision',grouping_proposal:'decision',
 catalog_ambiguity:'decision',weak_catalog_match:'decision',unsupported_format:'decision',needs_main_file:'blocked',no_catalog:'setup',
};

export const acceptable=(reason:ReviewReason)=>!['lookup_failed','needs_main_file','no_catalog','grouping_proposal'].includes(reason);
